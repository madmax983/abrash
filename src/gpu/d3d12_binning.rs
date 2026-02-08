//! D3D12 compute shader pipeline for triangle binning
//!
//! This module implements a simplified GPU binning pipeline that uses
//! root constants instead of descriptor heaps for ease of implementation.

#![cfg(feature = "gpu-binning")]

use super::{
    buffers::{BufferType, GpuBuffer},
    d3d12_device::{D3D12Device, GpuError},
};
use windows::{
    Win32::{
        Foundation::CloseHandle,
        Graphics::{Direct3D::ID3DBlob, Direct3D12::*},
        System::Threading::{CreateEventW, INFINITE, WaitForSingleObject},
    },
    core::Interface,
};

/// GPU compute binning pipeline
pub struct GpuBinner {
    device: D3D12Device,
    command_list: ID3D12GraphicsCommandList,
    root_signature: ID3D12RootSignature,
    compute_pso: ID3D12PipelineState,
    fence: ID3D12Fence,
    fence_value: u64,

    // GPU buffers
    triangle_upload: GpuBuffer,
    tile_bins_uav: GpuBuffer,
    bins_readback: GpuBuffer,

    // Descriptor heap for SRV/UAV
    descriptor_heap: ID3D12DescriptorHeap,
    descriptor_size: u32,

    // Constants
    tiles_x: u32,
    tiles_y: u32,
    tile_size: u32,
    max_triangles: usize,
    width: u32,
    height: u32,

    // Two-level hierarchical binning (optional)
    two_level_enabled: bool,
    coarse_bin_size: u32,
    coarse_bins_x: u32,
    coarse_bins_y: u32,
    coarse_bins_uav: Option<GpuBuffer>,
    coarse_bins_readback: Option<GpuBuffer>,
    coarse_binning_pso: Option<ID3D12PipelineState>,
    visible_bins_upload: Option<GpuBuffer>,
    fine_binning_pso: Option<ID3D12PipelineState>,
}

impl GpuBinner {
    /// Create a new GPU binning pipeline
    ///
    /// # Arguments
    /// * `width` - Framebuffer width in pixels
    /// * `height` - Framebuffer height in pixels
    /// * `tile_size` - Tile size (typically 32)
    /// * `max_triangles` - Max triangles per batch (typically 1000)
    pub fn new(
        width: u32,
        height: u32,
        tile_size: u32,
        max_triangles: usize,
    ) -> Result<Self, GpuError> {
        let tiles_x = (width + tile_size - 1) / tile_size;
        let tiles_y = (height + tile_size - 1) / tile_size;
        let tile_count = (tiles_x * tiles_y) as usize;

        // Create device
        let device = D3D12Device::new()?;

        // Create descriptor heap for SRV + UAV
        let descriptor_heap: ID3D12DescriptorHeap = unsafe {
            let desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                NumDescriptors: 2, // SRV + UAV
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                NodeMask: 0,
            };
            device
                .raw()
                .CreateDescriptorHeap(&desc)
                .map_err(GpuError::DeviceCreation)?
        };

        let descriptor_size = unsafe {
            device
                .raw()
                .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
        };

        // Allocate GPU buffers
        let triangle_buffer_size = max_triangles * std::mem::size_of::<PreparedTriangleGpu>();
        let bin_buffer_size = tile_count * std::mem::size_of::<TileBinGpu>();

        let triangle_upload = unsafe {
            GpuBuffer::new(device.raw(), BufferType::Upload, triangle_buffer_size)
                .map_err(GpuError::DeviceCreation)?
        };

        let tile_bins_uav = unsafe {
            GpuBuffer::new(device.raw(), BufferType::Uav, bin_buffer_size)
                .map_err(GpuError::DeviceCreation)?
        };

        let bins_readback = unsafe {
            GpuBuffer::new(device.raw(), BufferType::Readback, bin_buffer_size)
                .map_err(GpuError::DeviceCreation)?
        };

        // Create SRV for triangle buffer
        unsafe {
            let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                    Buffer: D3D12_BUFFER_SRV {
                        FirstElement: 0,
                        NumElements: max_triangles as u32,
                        StructureByteStride: std::mem::size_of::<PreparedTriangleGpu>() as u32,
                        Flags: D3D12_BUFFER_SRV_FLAG_NONE,
                    },
                },
            };

            let cpu_handle = descriptor_heap.GetCPUDescriptorHandleForHeapStart();
            device.raw().CreateShaderResourceView(
                triangle_upload.resource(),
                Some(&srv_desc),
                cpu_handle,
            );
        }

        // Create UAV for tile bins buffer
        unsafe {
            let uav_desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                ViewDimension: D3D12_UAV_DIMENSION_BUFFER,
                Anonymous: D3D12_UNORDERED_ACCESS_VIEW_DESC_0 {
                    Buffer: D3D12_BUFFER_UAV {
                        FirstElement: 0,
                        NumElements: tile_count as u32,
                        StructureByteStride: std::mem::size_of::<TileBinGpu>() as u32,
                        CounterOffsetInBytes: 0,
                        Flags: D3D12_BUFFER_UAV_FLAG_NONE,
                    },
                },
            };

            let mut cpu_handle = descriptor_heap.GetCPUDescriptorHandleForHeapStart();
            cpu_handle.ptr += descriptor_size as usize;
            device.raw().CreateUnorderedAccessView(
                tile_bins_uav.resource(),
                None,
                Some(&uav_desc),
                cpu_handle,
            );
        }

        // Create root signature
        let root_signature = Self::create_root_signature(device.raw())?;

        // Create compute PSO
        let shader_bytecode = include_bytes!("../../shaders/bin_triangles.cso");
        let compute_pso = Self::create_compute_pso(device.raw(), &root_signature, shader_bytecode)?;

        // Create command list
        let command_list: ID3D12GraphicsCommandList = unsafe {
            device
                .raw()
                .CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_COMPUTE,
                    device.command_allocator().raw(),
                    Some(&compute_pso),
                )
                .map_err(GpuError::DeviceCreation)?
        };

        // Close command list initially
        unsafe {
            command_list.Close().map_err(GpuError::DeviceCreation)?;
        }

        // Create fence
        let fence = unsafe {
            device
                .raw()
                .CreateFence(0, D3D12_FENCE_FLAG_NONE)
                .map_err(GpuError::DeviceCreation)?
        };

        Ok(Self {
            device,
            command_list,
            root_signature,
            compute_pso,
            fence,
            fence_value: 0,
            triangle_upload,
            tile_bins_uav,
            bins_readback,
            descriptor_heap,
            descriptor_size,
            tiles_x,
            tiles_y,
            tile_size,
            max_triangles,
            width,
            height,
            // Two-level binning (not enabled by default)
            two_level_enabled: false,
            coarse_bin_size: 128,
            coarse_bins_x: 0,
            coarse_bins_y: 0,
            coarse_bins_uav: None,
            coarse_bins_readback: None,
            coarse_binning_pso: None,
            visible_bins_upload: None,
            fine_binning_pso: None,
        })
    }

    /// Create root signature with descriptor table + constants
    fn create_root_signature(device: &ID3D12Device) -> Result<ID3D12RootSignature, GpuError> {
        unsafe {
            let ranges = [
                D3D12_DESCRIPTOR_RANGE {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
                    NumDescriptors: 1,
                    BaseShaderRegister: 0,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: 0,
                },
                D3D12_DESCRIPTOR_RANGE {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_UAV,
                    NumDescriptors: 1,
                    BaseShaderRegister: 0,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: 1,
                },
            ];

            let root_params = [
                // Descriptor table for SRV + UAV
                D3D12_ROOT_PARAMETER {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                    Anonymous: D3D12_ROOT_PARAMETER_0 {
                        DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                            NumDescriptorRanges: 2,
                            pDescriptorRanges: ranges.as_ptr(),
                        },
                    },
                    ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
                },
                // Root constants (4 u32: triangle_count, tiles_x, tiles_y, tile_size)
                D3D12_ROOT_PARAMETER {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                    Anonymous: D3D12_ROOT_PARAMETER_0 {
                        Constants: D3D12_ROOT_CONSTANTS {
                            ShaderRegister: 0,
                            RegisterSpace: 0,
                            Num32BitValues: 4,
                        },
                    },
                    ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
                },
            ];

            let desc = D3D12_ROOT_SIGNATURE_DESC {
                NumParameters: root_params.len() as u32,
                pParameters: root_params.as_ptr(),
                NumStaticSamplers: 0,
                pStaticSamplers: std::ptr::null(),
                Flags: D3D12_ROOT_SIGNATURE_FLAG_NONE,
            };

            let mut blob: Option<ID3DBlob> = None;
            D3D12SerializeRootSignature(&desc, D3D_ROOT_SIGNATURE_VERSION_1, &mut blob, None)
                .map_err(GpuError::DeviceCreation)?;

            let blob = blob.ok_or_else(|| {
                GpuError::DeviceCreation(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32),
                ))
            })?;

            let data = std::slice::from_raw_parts(
                blob.GetBufferPointer() as *const u8,
                blob.GetBufferSize(),
            );

            device
                .CreateRootSignature(0, data)
                .map_err(GpuError::DeviceCreation)
        }
    }

    /// Create compute pipeline state
    fn create_compute_pso(
        device: &ID3D12Device,
        root_signature: &ID3D12RootSignature,
        shader_bytecode: &[u8],
    ) -> Result<ID3D12PipelineState, GpuError> {
        let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: unsafe { std::mem::transmute_copy(root_signature) },
            CS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: shader_bytecode.as_ptr() as *const _,
                BytecodeLength: shader_bytecode.len(),
            },
            NodeMask: 0,
            CachedPSO: D3D12_CACHED_PIPELINE_STATE::default(),
            Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
        };

        unsafe {
            device
                .CreateComputePipelineState(&desc)
                .map_err(GpuError::DeviceCreation)
        }
    }

    /// Bin triangles using GPU compute
    ///
    /// # Arguments
    /// * `triangles` - Slice of PreparedTriangle structs to bin
    /// * `tile_bins` - Output vector to populate with binned triangle indices
    pub fn bin_triangles(
        &mut self,
        triangles: &[crate::tile_renderer::PreparedTriangle],
        tile_bins: &mut Vec<Vec<usize>>,
    ) -> Result<(), GpuError> {
        if triangles.is_empty() {
            return Ok(());
        }

        if triangles.len() > self.max_triangles {
            return Err(GpuError::DeviceCreation(
                windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32), // E_INVALIDARG
                ),
            ));
        }

        // Upload triangles
        self.upload_triangles(triangles)?;

        // Dispatch compute shader
        self.dispatch_compute(triangles.len() as u32)?;

        // Readback results
        self.readback_bins(tile_bins)?;

        Ok(())
    }

    /// Enable two-level hierarchical binning
    ///
    /// Allocates coarse binning buffers and compiles coarse binning shader.
    /// After calling this, use bin_triangles_two_level() instead of bin_triangles().
    pub fn enable_two_level_binning(&mut self) -> Result<(), GpuError> {
        if self.two_level_enabled {
            return Ok(()); // Already enabled
        }

        // Calculate coarse bin dimensions
        self.coarse_bins_x = (self.width + self.coarse_bin_size - 1) / self.coarse_bin_size;
        self.coarse_bins_y = (self.height + self.coarse_bin_size - 1) / self.coarse_bin_size;
        let coarse_bin_count = (self.coarse_bins_x * self.coarse_bins_y) as usize;

        // Allocate coarse bins UAV buffer
        let coarse_bin_size = std::mem::size_of::<CoarseBinGpu>();
        let coarse_bins_buffer_size = coarse_bin_count * coarse_bin_size;

        let coarse_bins_uav = unsafe {
            GpuBuffer::new(self.device.raw(), BufferType::Uav, coarse_bins_buffer_size)
                .map_err(GpuError::DeviceCreation)?
        };

        // Allocate coarse bins readback buffer
        let coarse_bins_readback = unsafe {
            GpuBuffer::new(
                self.device.raw(),
                BufferType::Readback,
                coarse_bins_buffer_size,
            )
            .map_err(GpuError::DeviceCreation)?
        };

        // Load and compile coarse binning shader
        let coarse_shader = include_bytes!("../../shaders/bin_coarse.cso");
        let coarse_binning_pso =
            Self::create_compute_pso(self.device.raw(), &self.root_signature, coarse_shader)?;

        // Load and compile fine binning shader
        let fine_shader = include_bytes!("../../shaders/bin_fine.cso");
        let fine_binning_pso =
            Self::create_compute_pso(self.device.raw(), &self.root_signature, fine_shader)?;

        // Allocate visible bins upload buffer (max size: all coarse bins could be visible)
        let visible_bins_buffer_size = coarse_bin_count * coarse_bin_size;
        let visible_bins_upload = unsafe {
            GpuBuffer::new(
                self.device.raw(),
                BufferType::Upload,
                visible_bins_buffer_size,
            )
            .map_err(GpuError::DeviceCreation)?
        };

        self.coarse_bins_uav = Some(coarse_bins_uav);
        self.coarse_bins_readback = Some(coarse_bins_readback);
        self.coarse_binning_pso = Some(coarse_binning_pso);
        self.visible_bins_upload = Some(visible_bins_upload);
        self.fine_binning_pso = Some(fine_binning_pso);
        self.two_level_enabled = true;

        Ok(())
    }

    /// Check if two-level binning is enabled
    pub fn is_two_level_enabled(&self) -> bool {
        self.two_level_enabled
    }

    /// Two-level binning: coarse → Hi-Z cull → fine
    ///
    /// This is the main entry point for two-level hierarchical binning.
    /// Must call `enable_two_level_binning()` first.
    ///
    /// # Returns
    /// Stats about culling effectiveness
    pub fn bin_triangles_two_level(
        &mut self,
        triangles: &[crate::tile_renderer::PreparedTriangle],
        hiz_buffer: Option<&crate::hiz_buffer::HiZBuffer>,
        tile_bins: &mut Vec<Vec<usize>>,
    ) -> Result<TwoLevelBinningStats, GpuError> {
        if !self.two_level_enabled {
            return Err(GpuError::DeviceCreation(
                windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0057u32 as i32)),
            ));
        }

        if triangles.is_empty() {
            return Ok(TwoLevelBinningStats::default());
        }

        // Phase 1: Upload triangles
        self.upload_triangles(triangles)?;

        // Phase 2a: Coarse binning (GPU)
        let coarse_bins = self.dispatch_coarse_binning(triangles.len() as u32)?;

        // Phase 2b: Hi-Z culling (CPU)
        let visible_bins = self.cull_coarse_bins(&coarse_bins, hiz_buffer);
        let stats = TwoLevelBinningStats {
            total_coarse_bins: coarse_bins.len(),
            visible_coarse_bins: visible_bins.len(),
            culled_coarse_bins: coarse_bins.len() - visible_bins.len(),
        };

        // Phase 2c: Fine binning (GPU, visible bins only)
        self.dispatch_fine_binning(&visible_bins, tile_bins)?;

        Ok(stats)
    }

    /// Dispatch coarse binning compute shader
    fn dispatch_coarse_binning(
        &mut self,
        triangle_count: u32,
    ) -> Result<Vec<CoarseBinCpu>, GpuError> {
        let coarse_bins_uav = self.coarse_bins_uav.as_ref().ok_or_else(|| {
            GpuError::DeviceCreation(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x8007_0057u32 as i32,
            )))
        })?;
        let coarse_binning_pso = self.coarse_binning_pso.as_ref().ok_or_else(|| {
            GpuError::DeviceCreation(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x8007_0057u32 as i32,
            )))
        })?;

        unsafe {
            // Reset command list
            self.device
                .command_allocator()
                .reset()
                .map_err(GpuError::CommandAllocatorCreation)?;
            self.command_list
                .Reset(
                    self.device.command_allocator().raw(),
                    Some(coarse_binning_pso),
                )
                .map_err(GpuError::DeviceCreation)?;

            // Clear coarse bins UAV
            let clear_values = [0u32, 0, 0, 0];
            let gpu_uav_handle = unsafe {
                let mut handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as u64;
                handle
            };
            let cpu_uav_handle = unsafe {
                let mut handle = self.descriptor_heap.GetCPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as usize;
                handle
            };
            self.command_list.ClearUnorderedAccessViewUint(
                gpu_uav_handle,
                cpu_uav_handle,
                coarse_bins_uav.resource(),
                &clear_values,
                &[],
            );

            // Set pipeline state
            self.command_list.SetPipelineState(coarse_binning_pso);
            self.command_list
                .SetComputeRootSignature(&self.root_signature);

            // Set descriptor heap
            let heaps = [Some(self.descriptor_heap.clone())];
            self.command_list.SetDescriptorHeaps(&heaps);

            // Set descriptor table
            let gpu_handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
            self.command_list
                .SetComputeRootDescriptorTable(0, gpu_handle);

            // Set root constants (triangle_count, coarse_bins_x, coarse_bins_y, coarse_bin_size)
            let constants = [
                triangle_count,
                self.coarse_bins_x,
                self.coarse_bins_y,
                self.coarse_bin_size,
            ];
            self.command_list
                .SetComputeRoot32BitConstants(1, 4, constants.as_ptr() as *const _, 0);

            // Dispatch (64 threads per group)
            let thread_groups = (triangle_count + 63) / 64;
            self.command_list.Dispatch(thread_groups, 1, 1);

            // Copy UAV to readback
            let readback = self.coarse_bins_readback.as_ref().unwrap();
            self.command_list
                .CopyResource(readback.resource(), coarse_bins_uav.resource());

            // Execute
            self.command_list
                .Close()
                .map_err(GpuError::DeviceCreation)?;
            let cmd_lists = [Some(self.command_list.cast::<ID3D12CommandList>().unwrap())];
            self.device
                .command_queue()
                .raw()
                .ExecuteCommandLists(&cmd_lists);

            // Wait for completion
            self.fence_value += 1;
            self.device
                .command_queue()
                .raw()
                .Signal(&self.fence, self.fence_value)
                .map_err(GpuError::DeviceCreation)?;

            if self.fence.GetCompletedValue() < self.fence_value {
                let event =
                    CreateEventW(None, false, false, None).map_err(GpuError::DeviceCreation)?;
                self.fence
                    .SetEventOnCompletion(self.fence_value, event)
                    .map_err(GpuError::DeviceCreation)?;
                WaitForSingleObject(event, u32::MAX);
                CloseHandle(event).ok();
            }

            // Readback coarse bins
            let readback = self.coarse_bins_readback.as_mut().unwrap();
            let ptr = readback.map().map_err(GpuError::DeviceCreation)?;
            let bin_count = (self.coarse_bins_x * self.coarse_bins_y) as usize;
            let gpu_bins = std::slice::from_raw_parts(ptr as *const CoarseBinGpu, bin_count);

            let mut coarse_bins = Vec::with_capacity(bin_count);
            for (i, gpu_bin) in gpu_bins.iter().enumerate() {
                let count = gpu_bin.count.min(510) as usize;
                let mut bin = CoarseBinCpu {
                    bin_index: i,
                    triangle_indices: Vec::with_capacity(count),
                };
                for j in 0..count {
                    bin.triangle_indices
                        .push(gpu_bin.triangle_indices[j] as usize);
                }
                coarse_bins.push(bin);
            }

            readback.unmap();
            Ok(coarse_bins)
        }
    }

    /// Cull coarse bins using Hi-Z buffer (CPU)
    fn cull_coarse_bins(
        &self,
        coarse_bins: &[CoarseBinCpu],
        hiz_buffer: Option<&crate::hiz_buffer::HiZBuffer>,
    ) -> Vec<CoarseBinCpu> {
        let Some(hiz) = hiz_buffer else {
            // No Hi-Z, all bins visible
            return coarse_bins.to_vec();
        };

        coarse_bins
            .iter()
            .filter(|bin| {
                if bin.triangle_indices.is_empty() {
                    return false; // Skip empty bins
                }

                // Compute bin screen AABB
                let bin_x = (bin.bin_index % self.coarse_bins_x as usize) as u32;
                let bin_y = (bin.bin_index / self.coarse_bins_x as usize) as u32;
                let min_x = (bin_x * self.coarse_bin_size) as i32;
                let min_y = (bin_y * self.coarse_bin_size) as i32;
                let max_x = min_x + self.coarse_bin_size as i32 - 1;
                let max_y = min_y + self.coarse_bin_size as i32 - 1;

                // Query Hi-Z
                let bin_aabb = crate::hiz_buffer::AABB3D {
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_depth: 0.0, // Conservative: assume bin could contain any depth
                    max_depth: f32::INFINITY,
                };

                hiz.is_coarse_bin_visible(bin_aabb)
            })
            .cloned()
            .collect()
    }

    /// Dispatch fine binning compute shader (visible bins only)
    fn dispatch_fine_binning(
        &mut self,
        visible_bins: &[CoarseBinCpu],
        tile_bins: &mut Vec<Vec<usize>>,
    ) -> Result<(), GpuError> {
        if visible_bins.is_empty() {
            // No visible bins, clear all tile bins
            for bin in tile_bins.iter_mut() {
                bin.clear();
            }
            return Ok(());
        }

        let fine_binning_pso = self.fine_binning_pso.as_ref().ok_or_else(|| {
            GpuError::DeviceCreation(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x8007_0057u32 as i32,
            )))
        })?;

        unsafe {
            // Step 1: Upload visible bins to GPU
            let upload_buf = self.visible_bins_upload.as_mut().unwrap();
            let ptr = upload_buf.map().map_err(GpuError::DeviceCreation)?;

            // Convert CoarseBinCpu to VisibleCoarseBinGpu layout
            let visible_gpu: Vec<VisibleCoarseBinGpu> = visible_bins
                .iter()
                .map(|bin| {
                    let mut gpu_bin = VisibleCoarseBinGpu {
                        coarse_bin_index: bin.bin_index as u32,
                        triangle_count: bin.triangle_indices.len() as u32,
                        _padding: [0; 2],
                        triangle_indices: [0; 510],
                    };
                    for (i, &tri_idx) in bin.triangle_indices.iter().enumerate() {
                        gpu_bin.triangle_indices[i] = tri_idx as u32;
                    }
                    gpu_bin
                })
                .collect();

            std::ptr::copy_nonoverlapping(
                visible_gpu.as_ptr() as *const u8,
                ptr,
                visible_gpu.len() * std::mem::size_of::<VisibleCoarseBinGpu>(),
            );

            upload_buf.unmap();

            // Step 2: Reset command list
            self.device
                .command_allocator()
                .reset()
                .map_err(GpuError::CommandAllocatorCreation)?;
            self.command_list
                .Reset(
                    self.device.command_allocator().raw(),
                    Some(fine_binning_pso),
                )
                .map_err(GpuError::DeviceCreation)?;

            // Step 3: Clear tile bins UAV (same as coarse binning clear pattern)
            let clear_values = [0u32, 0, 0, 0];
            let gpu_uav_handle = {
                let mut handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as u64;
                handle
            };
            let cpu_uav_handle = {
                let mut handle = self.descriptor_heap.GetCPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as usize;
                handle
            };
            self.command_list.ClearUnorderedAccessViewUint(
                gpu_uav_handle,
                cpu_uav_handle,
                self.tile_bins_uav.resource(),
                &clear_values,
                &[],
            );

            // Step 4: Set pipeline state
            self.command_list.SetPipelineState(fine_binning_pso);
            self.command_list
                .SetComputeRootSignature(&self.root_signature);

            // Step 5: Set descriptor heap
            let heaps = [Some(self.descriptor_heap.clone())];
            self.command_list.SetDescriptorHeaps(&heaps);

            // Step 6: Set descriptor table
            let gpu_handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
            self.command_list
                .SetComputeRootDescriptorTable(0, gpu_handle);

            // Step 7: Set root constants (visible_bin_count, tiles_x, tiles_y, tile_size)
            let constants = [visible_bins.len() as u32, self.tiles_x, self.tiles_y, 32];
            self.command_list.SetComputeRoot32BitConstants(
                1,
                4,
                constants.as_ptr() as *const std::ffi::c_void,
                0,
            );

            // Step 8: Dispatch compute shader (one thread group per visible bin)
            let thread_groups = (visible_bins.len() as u32 + 63) / 64;
            self.command_list.Dispatch(thread_groups, 1, 1);

            // Step 9: Close and execute command list
            self.command_list
                .Close()
                .map_err(GpuError::DeviceCreation)?;

            let cmd_lists = [Some(
                self.command_list
                    .cast::<ID3D12CommandList>()
                    .map_err(GpuError::DeviceCreation)?,
            )];
            self.device
                .command_queue()
                .raw()
                .ExecuteCommandLists(&cmd_lists);

            // Step 10: Wait for GPU to finish (same pattern as dispatch_coarse_binning)
            self.fence_value += 1;
            self.device
                .command_queue()
                .raw()
                .Signal(&self.fence, self.fence_value)
                .map_err(GpuError::DeviceCreation)?;

            if self.fence.GetCompletedValue() < self.fence_value {
                let event = CreateEventW(None, false, false, None)
                    .map_err(|e| GpuError::DeviceCreation(e.into()))?;
                self.fence
                    .SetEventOnCompletion(self.fence_value, event)
                    .map_err(GpuError::DeviceCreation)?;
                let _ = WaitForSingleObject(event, INFINITE);
                let _ = CloseHandle(event);
            }

            // Step 11: Readback tile bins to CPU
            let ptr = self.bins_readback.map().map_err(GpuError::DeviceCreation)?;

            let tile_count = (self.tiles_x * self.tiles_y) as usize;
            for tile_idx in 0..tile_count {
                let offset = tile_idx * std::mem::size_of::<TileBinGpu>();
                let tile_bin_ptr = ptr.add(offset) as *const TileBinGpu;
                let tile_bin = &*tile_bin_ptr;

                tile_bins[tile_idx].clear();
                let count = tile_bin.count.min(256) as usize;
                for i in 0..count {
                    tile_bins[tile_idx].push(tile_bin.triangle_indices[i] as usize);
                }
            }

            self.bins_readback.unmap();
            Ok(())
        }
    }

    /// Upload triangle data to GPU
    fn upload_triangles(
        &mut self,
        triangles: &[crate::tile_renderer::PreparedTriangle],
    ) -> Result<(), GpuError> {
        unsafe {
            let ptr = self
                .triangle_upload
                .map()
                .map_err(GpuError::DeviceCreation)?;

            // Convert Rust PreparedTriangle to GPU layout
            let gpu_triangles: Vec<PreparedTriangleGpu> = triangles
                .iter()
                .map(|t| PreparedTriangleGpu {
                    p0: [t.p0.x as f32, t.p0.y as f32, t.p0.z],
                    p1: [t.p1.x as f32, t.p1.y as f32, t.p1.z],
                    p2: [t.p2.x as f32, t.p2.y as f32, t.p2.z],
                    p0_fixed: [t.p0_fixed.x, t.p0_fixed.y, t.p0_fixed.z],
                    p1_fixed: [t.p1_fixed.x, t.p1_fixed.y, t.p1_fixed.z],
                    p2_fixed: [t.p2_fixed.x, t.p2_fixed.y, t.p2_fixed.z],
                    dz_dx: t.dz_dx,
                    long_edge_is_left: u32::from(t.long_edge_is_left),
                    color: t.color,
                    aabb_min_x: t.aabb_min_x,
                    aabb_min_y: t.aabb_min_y,
                    aabb_max_x: t.aabb_max_x,
                    aabb_max_y: t.aabb_max_y,
                    min_depth: t.min_depth,
                    max_depth: t.max_depth,
                })
                .collect();

            // Copy to GPU
            std::ptr::copy_nonoverlapping(
                gpu_triangles.as_ptr(),
                ptr as *mut PreparedTriangleGpu,
                gpu_triangles.len(),
            );

            self.triangle_upload.unmap();
        }

        Ok(())
    }

    /// Dispatch compute shader
    fn dispatch_compute(&mut self, triangle_count: u32) -> Result<(), GpuError> {
        unsafe {
            // Reset command allocator and list
            self.device
                .command_allocator()
                .reset()
                .map_err(GpuError::CommandAllocatorCreation)?;
            self.command_list
                .Reset(
                    self.device.command_allocator().raw(),
                    Some(&self.compute_pso),
                )
                .map_err(GpuError::DeviceCreation)?;

            // Set pipeline state
            self.command_list.SetPipelineState(&self.compute_pso);
            self.command_list
                .SetComputeRootSignature(&self.root_signature);

            // Set descriptor heap
            let heaps = [Some(self.descriptor_heap.clone())];
            self.command_list.SetDescriptorHeaps(&heaps);

            // Set descriptor table
            let gpu_handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
            self.command_list
                .SetComputeRootDescriptorTable(0, gpu_handle);

            // Clear UAV buffer before binning (zero all counts)
            let clear_values = [0u32, 0, 0, 0];
            let gpu_uav_handle = unsafe {
                let mut handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as u64; // Advance to UAV (SRV is at offset 0)
                handle
            };
            let cpu_uav_handle = unsafe {
                let mut handle = self.descriptor_heap.GetCPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as usize; // Advance to UAV
                handle
            };
            self.command_list.ClearUnorderedAccessViewUint(
                gpu_uav_handle,
                cpu_uav_handle,
                self.tile_bins_uav.resource(),
                &clear_values,
                &[],
            );

            // Set root constants
            let constants = [triangle_count, self.tiles_x, self.tiles_y, self.tile_size];
            self.command_list
                .SetComputeRoot32BitConstants(1, 4, constants.as_ptr() as *const _, 0);

            // Dispatch compute shader (64 threads per group)
            let thread_groups = (triangle_count + 63) / 64;
            self.command_list.Dispatch(thread_groups, 1, 1);

            // Copy UAV to readback buffer
            self.command_list
                .CopyResource(self.bins_readback.resource(), self.tile_bins_uav.resource());

            // Close and execute
            self.command_list
                .Close()
                .map_err(GpuError::DeviceCreation)?;

            let cmd_lists = [Some(self.command_list.cast::<ID3D12CommandList>().unwrap())];
            self.device
                .command_queue()
                .raw()
                .ExecuteCommandLists(&cmd_lists);

            // Wait for completion
            self.fence_value += 1;
            self.device
                .command_queue()
                .raw()
                .Signal(&self.fence, self.fence_value)
                .map_err(GpuError::DeviceCreation)?;

            if self.fence.GetCompletedValue() < self.fence_value {
                let event = CreateEventW(None, false, false, None)
                    .map_err(|e: windows::core::Error| GpuError::DeviceCreation(e))?;

                self.fence
                    .SetEventOnCompletion(self.fence_value, event)
                    .map_err(GpuError::DeviceCreation)?;

                WaitForSingleObject(event, u32::MAX);
                let _ = CloseHandle(event);
            }
        }

        Ok(())
    }

    /// Readback binning results from GPU
    fn readback_bins(&mut self, tile_bins: &mut Vec<Vec<usize>>) -> Result<(), GpuError> {
        unsafe {
            let ptr = self.bins_readback.map().map_err(GpuError::DeviceCreation)?;
            let gpu_bins = std::slice::from_raw_parts(ptr as *const TileBinGpu, tile_bins.len());

            // Clear and populate tile bins
            for (i, tile_bin) in tile_bins.iter_mut().enumerate() {
                tile_bin.clear();
                let gpu_bin = &gpu_bins[i];
                let count = gpu_bin.count.min(510) as usize;
                for j in 0..count {
                    tile_bin.push(gpu_bin.triangle_indices[j] as usize);
                }
            }

            self.bins_readback.unmap();
        }

        Ok(())
    }
}

/// Statistics from two-level hierarchical binning
#[derive(Debug, Default, Clone)]
pub struct TwoLevelBinningStats {
    pub total_coarse_bins: usize,
    pub visible_coarse_bins: usize,
    pub culled_coarse_bins: usize,
}

/// CPU representation of a coarse bin after GPU readback
#[derive(Debug, Clone)]
struct CoarseBinCpu {
    bin_index: usize,
    triangle_indices: Vec<usize>,
}

/// GPU representation of PreparedTriangle (80 bytes)
#[repr(C)]
#[derive(Copy, Clone)]
struct PreparedTriangleGpu {
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p0_fixed: [i32; 3],
    p1_fixed: [i32; 3],
    p2_fixed: [i32; 3],
    dz_dx: f32,
    long_edge_is_left: u32,
    color: u32,
    aabb_min_x: i32,
    aabb_min_y: i32,
    aabb_max_x: i32,
    aabb_max_y: i32,
    min_depth: f32,
    max_depth: f32,
}

/// GPU representation of TileBin (2044 bytes: 4 + 510*4, under D3D12 2048-byte limit)
#[repr(C)]
struct TileBinGpu {
    count: u32,
    triangle_indices: [u32; 510],
}

/// GPU representation of CoarseBin (2044 bytes: 4 + 510*4, same as TileBin)
/// Depth range computed on CPU after readback to avoid atomic float issues
#[repr(C)]
struct CoarseBinGpu {
    count: u32,
    triangle_indices: [u32; 510],
}

/// GPU representation of VisibleCoarseBin for fine binning pass
/// Contains coarse bin index + triangles for bins that passed Hi-Z culling
#[repr(C)]
struct VisibleCoarseBinGpu {
    coarse_bin_index: u32,
    triangle_count: u32,
    _padding: [u32; 2], // Align to 16 bytes
    triangle_indices: [u32; 510],
}
