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

/// Minimal screen-space vertex input used by GPU triangle binning.
#[derive(Debug, Clone, Copy)]
pub struct ScreenVertexInput {
    pub x: i32,
    pub y: i32,
    pub z: f32,
}

/// Fixed-point vertex used by GPU edge setup.
#[derive(Debug, Clone, Copy)]
pub struct VertexFixedInput {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Triangle payload consumed by GPU binning shaders.
#[derive(Debug, Clone, Copy)]
pub struct PreparedTriangleInput {
    pub p0: ScreenVertexInput,
    pub p1: ScreenVertexInput,
    pub p2: ScreenVertexInput,
    pub p0_fixed: VertexFixedInput,
    pub p1_fixed: VertexFixedInput,
    pub p2_fixed: VertexFixedInput,
    pub dz_dx: f32,
    pub long_edge_is_left: bool,
    pub color: u32,
    pub aabb_min_x: i32,
    pub aabb_min_y: i32,
    pub aabb_max_x: i32,
    pub aabb_max_y: i32,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// Axis-aligned bounding box used for Hi-Z visibility queries.
#[derive(Debug, Clone, Copy)]
pub struct Aabb3d {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// Adapter trait for coarse-bin visibility checks against a Hi-Z structure.

/// Adapter trait for writing GPU-built Hi-Z pyramid levels back to CPU storage.

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
        let shader_bytecode = include_bytes!("../shaders/bin_triangles.cso");
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
    /// * `heads`, `tails`, `nexts`, `tris` - Output flattened linked list structure
    pub fn bin_triangles(
        &mut self,
        triangles: &[PreparedTriangleInput],
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
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
        self.readback_bins(heads, tails, nexts, tris)?;

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
        let coarse_shader = include_bytes!("../shaders/bin_coarse.cso");
        let coarse_binning_pso =
            Self::create_compute_pso(self.device.raw(), &self.root_signature, coarse_shader)?;

        // Load and compile fine binning shader
        let fine_shader = include_bytes!("../shaders/bin_fine.cso");
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
        triangles: &[PreparedTriangleInput],
        hiz_buffer: Option<&impl Fn(Aabb3d) -> bool>,
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
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
        self.dispatch_fine_binning(&visible_bins, heads, tails, nexts, tris)?;

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
        hiz_buffer: Option<&impl Fn(Aabb3d) -> bool>,
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
                let bin_aabb = Aabb3d {
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_depth: 0.0, // Conservative: assume bin could contain any depth
                    max_depth: f32::INFINITY,
                };

                hiz(bin_aabb)
            })
            .cloned()
            .collect()
    }

    /// Dispatch fine binning compute shader (visible bins only)
    fn dispatch_fine_binning(
        &mut self,
        visible_bins: &[CoarseBinCpu],
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
    ) -> Result<(), GpuError> {
        if visible_bins.is_empty() {
            // No visible bins, clear all tile bins
            heads.fill(u32::MAX);
            tails.fill(u32::MAX);
            nexts.clear();
            tris.clear();
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

            heads.fill(u32::MAX);
            tails.fill(u32::MAX);
            nexts.clear();
            tris.clear();

            let tile_count = (self.tiles_x * self.tiles_y) as usize;
            for tile_idx in 0..tile_count {
                let offset = tile_idx * std::mem::size_of::<TileBinGpu>();
                let tile_bin_ptr = ptr.add(offset) as *const TileBinGpu;
                let tile_bin = &*tile_bin_ptr;

                let count = tile_bin.count.min(256) as usize;
                if count > 0 {
                    for i in 0..count {
                        let tri_idx = tile_bin.triangle_indices[i];

                        let node_idx = tris.len() as u32;
                        tris.push(tri_idx);
                        nexts.push(u32::MAX);

                        let head = heads[tile_idx];
                        if head == u32::MAX {
                            heads[tile_idx] = node_idx;
                        } else {
                            let tail = tails[tile_idx];
                            nexts[tail as usize] = node_idx;
                        }
                        tails[tile_idx] = node_idx;
                    }
                }
            }

            self.bins_readback.unmap();
            Ok(())
        }
    }

    /// Upload triangle data to GPU
    fn upload_triangles(&mut self, triangles: &[PreparedTriangleInput]) -> Result<(), GpuError> {
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
            let gpu_uav_handle = {
                let mut handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
                handle.ptr += self.descriptor_size as u64; // Advance to UAV (SRV is at offset 0)
                handle
            };
            let cpu_uav_handle = {
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
    fn readback_bins(
        &mut self,
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
    ) -> Result<(), GpuError> {
        unsafe {
            let ptr = self.bins_readback.map().map_err(GpuError::DeviceCreation)?;
            let gpu_bins = std::slice::from_raw_parts(ptr as *const TileBinGpu, heads.len());

            heads.fill(u32::MAX);
            tails.fill(u32::MAX);
            nexts.clear();
            tris.clear();

            // Clear and populate tile bins
            for (tile_idx, gpu_bin) in gpu_bins.iter().enumerate() {
                let count = gpu_bin.count.min(510) as usize;
                if count > 0 {
                    for j in 0..count {
                        let tri_idx = gpu_bin.triangle_indices[j];

                        let node_idx = tris.len() as u32;
                        tris.push(tri_idx);
                        nexts.push(u32::MAX);

                        let head = heads[tile_idx];
                        if head == u32::MAX {
                            heads[tile_idx] = node_idx;
                        } else {
                            let tail = tails[tile_idx];
                            nexts[tail as usize] = node_idx;
                        }
                        tails[tile_idx] = node_idx;
                    }
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

/// GPU Hi-Z pyramid builder using D3D12 compute shaders
///
/// Builds the hierarchical z-buffer pyramid on GPU using texture resources
/// and compute shader dispatches. The pyramid is built iteratively, with
/// each level performing a 2×2 min-reduction from the level below.
///
/// # Architecture
/// - Upload zbuffer to GPU texture (level 0)
/// - Dispatch compute shader for each pyramid level (1..N)
/// - Download complete pyramid back to CPU for occlusion queries
///
/// # Performance
/// Expected 10-15× faster than CPU pyramid build for 1080p+ resolutions
pub struct GpuHiZBuilder {
    device: D3D12Device,
    command_list: ID3D12GraphicsCommandList,
    root_signature: ID3D12RootSignature,
    compute_pso: ID3D12PipelineState,
    fence: ID3D12Fence,
    fence_value: u64,

    // GPU resources for Hi-Z pyramid
    zbuffer_upload: ID3D12Resource,      // Upload heap for CPU zbuffer
    zbuffer_texture: ID3D12Resource,     // Default heap, level 0 of pyramid
    pyramid_levels: Vec<ID3D12Resource>, // UAV textures for levels 1..N
    pyramid_readback: ID3D12Resource,    // Readback heap for download

    // Descriptor heap for SRV/UAV per level
    descriptor_heap: ID3D12DescriptorHeap,
    descriptor_size: u32,

    // Pyramid dimensions
    width: u32,
    height: u32,
    level_count: u32,
}

impl GpuHiZBuilder {
    /// Create a new GPU Hi-Z pyramid builder
    ///
    /// # Arguments
    /// * `width` - Framebuffer width in pixels
    /// * `height` - Framebuffer height in pixels
    ///
    /// # Returns
    /// Result containing the builder or GPU error
    pub fn new(width: u32, height: u32) -> Result<Self, GpuError> {
        // Calculate pyramid level count (same as HiZBuffer)
        let max_dim = width.max(height) as f32;
        let level_count = max_dim.log2().ceil() as u32 + 1;

        // Create D3D12 device
        let device = D3D12Device::new()?;

        // Create descriptor heap (SRV + UAV for each level pair)
        // We need max 2 descriptors per dispatch (source SRV + dest UAV)
        let descriptor_heap: ID3D12DescriptorHeap = unsafe {
            let desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                NumDescriptors: (level_count * 2) as u32, // SRV + UAV per level
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

        // Create zbuffer upload buffer (CPU -> GPU)
        let zbuffer_size = (width * height) as usize * std::mem::size_of::<f32>();
        let zbuffer_upload = Self::create_upload_buffer(device.raw(), zbuffer_size)?;

        // Create zbuffer texture (level 0, default heap)
        let zbuffer_texture = Self::create_texture(device.raw(), width, height)?;

        // Create pyramid levels (1..level_count)
        let mut pyramid_levels = Vec::with_capacity((level_count - 1) as usize);
        for level_idx in 1..level_count {
            let scale = 1u32 << level_idx;
            let level_width = width.div_ceil(scale);
            let level_height = height.div_ceil(scale);
            let level_texture = Self::create_texture(device.raw(), level_width, level_height)?;
            pyramid_levels.push(level_texture);
        }

        // Create readback buffer (GPU -> CPU, all levels)
        let mut total_pyramid_size = zbuffer_size; // Level 0
        for level_idx in 1..level_count {
            let scale = 1u32 << level_idx;
            let level_width = width.div_ceil(scale);
            let level_height = height.div_ceil(scale);
            total_pyramid_size +=
                (level_width * level_height) as usize * std::mem::size_of::<f32>();
        }
        let pyramid_readback = Self::create_readback_buffer(device.raw(), total_pyramid_size)?;

        // Create descriptor views (will be filled in create_descriptors)
        Self::create_descriptors(
            device.raw(),
            &descriptor_heap,
            descriptor_size,
            &zbuffer_texture,
            &pyramid_levels,
            width,
            height,
            level_count,
        )?;

        // Create root signature
        let root_signature = Self::create_root_signature(device.raw())?;

        // Create compute PSO (shader bytecode will be loaded from compiled .cso)
        let shader_bytecode = include_bytes!("../shaders/build_hiz_level.cso");
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

        // Create fence for GPU synchronization
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
            zbuffer_upload,
            zbuffer_texture,
            pyramid_levels,
            pyramid_readback,
            descriptor_heap,
            descriptor_size,
            width,
            height,
            level_count,
        })
    }

    /// Calculate D3D12 aligned row pitch (must be multiple of 256 bytes)
    fn aligned_row_pitch(width: u32, bytes_per_pixel: u32) -> u32 {
        const D3D12_TEXTURE_DATA_PITCH_ALIGNMENT: u32 = 256;
        let pitch = width * bytes_per_pixel;
        ((pitch + D3D12_TEXTURE_DATA_PITCH_ALIGNMENT - 1) / D3D12_TEXTURE_DATA_PITCH_ALIGNMENT)
            * D3D12_TEXTURE_DATA_PITCH_ALIGNMENT
    }

    /// Create upload buffer (CPU -> GPU)
    fn create_upload_buffer(
        device: &ID3D12Device,
        size: usize,
    ) -> Result<ID3D12Resource, GpuError> {
        unsafe {
            let heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_UPLOAD,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 0,
                VisibleNodeMask: 0,
            };

            let resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Alignment: 0,
                Width: size as u64,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                Flags: D3D12_RESOURCE_FLAG_NONE,
            };

            let mut resource: Option<ID3D12Resource> = None;
            device
                .CreateCommittedResource(
                    &heap_props,
                    D3D12_HEAP_FLAG_NONE,
                    &resource_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut resource,
                )
                .map_err(GpuError::DeviceCreation)?;

            resource.ok_or_else(|| {
                GpuError::DeviceCreation(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32),
                ))
            })
        }
    }

    /// Create texture resource (default heap, R32_FLOAT)
    fn create_texture(
        device: &ID3D12Device,
        width: u32,
        height: u32,
    ) -> Result<ID3D12Resource, GpuError> {
        unsafe {
            let heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 0,
                VisibleNodeMask: 0,
            };

            let resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Alignment: 0,
                Width: width as u64,
                Height: height,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
            };

            let mut resource: Option<ID3D12Resource> = None;
            device
                .CreateCommittedResource(
                    &heap_props,
                    D3D12_HEAP_FLAG_NONE,
                    &resource_desc,
                    D3D12_RESOURCE_STATE_COMMON,
                    None,
                    &mut resource,
                )
                .map_err(GpuError::DeviceCreation)?;

            resource.ok_or_else(|| {
                GpuError::DeviceCreation(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32),
                ))
            })
        }
    }

    /// Create readback buffer (GPU -> CPU)
    fn create_readback_buffer(
        device: &ID3D12Device,
        size: usize,
    ) -> Result<ID3D12Resource, GpuError> {
        unsafe {
            let heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_READBACK,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 0,
                VisibleNodeMask: 0,
            };

            let resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Alignment: 0,
                Width: size as u64,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                Flags: D3D12_RESOURCE_FLAG_NONE,
            };

            let mut resource: Option<ID3D12Resource> = None;
            device
                .CreateCommittedResource(
                    &heap_props,
                    D3D12_HEAP_FLAG_NONE,
                    &resource_desc,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    None,
                    &mut resource,
                )
                .map_err(GpuError::DeviceCreation)?;

            resource.ok_or_else(|| {
                GpuError::DeviceCreation(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32),
                ))
            })
        }
    }

    /// Create SRV and UAV descriptors for all pyramid levels
    #[allow(clippy::too_many_arguments)]
    fn create_descriptors(
        device: &ID3D12Device,
        descriptor_heap: &ID3D12DescriptorHeap,
        descriptor_size: u32,
        zbuffer_texture: &ID3D12Resource,
        pyramid_levels: &[ID3D12Resource],
        _width: u32,
        _height: u32,
        _level_count: u32,
    ) -> Result<(), GpuError> {
        unsafe {
            let mut cpu_handle = descriptor_heap.GetCPUDescriptorHandleForHeapStart();

            // Create SRV for level 0 (zbuffer texture - source for level 1 build)
            let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                        PlaneSlice: 0,
                        ResourceMinLODClamp: 0.0,
                    },
                },
            };
            device.CreateShaderResourceView(zbuffer_texture, Some(&srv_desc), cpu_handle);
            cpu_handle.ptr += descriptor_size as usize;

            // Create UAV for level 1 (pyramid_levels[0] - destination for level 1 build)
            // NOTE: This UAV points to pyramid_levels[0], NOT zbuffer_texture
            // Only create if pyramid has levels beyond level 0
            if !pyramid_levels.is_empty() {
                let uav_desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D12_UAV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D12_UNORDERED_ACCESS_VIEW_DESC_0 {
                        Texture2D: D3D12_TEX2D_UAV {
                            MipSlice: 0,
                            PlaneSlice: 0,
                        },
                    },
                };
                device.CreateUnorderedAccessView(
                    &pyramid_levels[0],
                    None,
                    Some(&uav_desc),
                    cpu_handle,
                );
                cpu_handle.ptr += descriptor_size as usize;
            }

            // Create SRV/UAV for pyramid levels 1..N
            for (idx, level_texture) in pyramid_levels.iter().enumerate() {
                let _level_idx = idx + 1;

                // SRV for this level (as source for next level)
                let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                    Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                    Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D12_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                            PlaneSlice: 0,
                            ResourceMinLODClamp: 0.0,
                        },
                    },
                };
                device.CreateShaderResourceView(level_texture, Some(&srv_desc), cpu_handle);
                cpu_handle.ptr += descriptor_size as usize;

                // UAV for this level (as destination)
                let uav_desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D12_UAV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D12_UNORDERED_ACCESS_VIEW_DESC_0 {
                        Texture2D: D3D12_TEX2D_UAV {
                            MipSlice: 0,
                            PlaneSlice: 0,
                        },
                    },
                };
                device.CreateUnorderedAccessView(level_texture, None, Some(&uav_desc), cpu_handle);
                cpu_handle.ptr += descriptor_size as usize;
            }
        }

        Ok(())
    }

    /// Create root signature for Hi-Z pyramid build shader
    fn create_root_signature(device: &ID3D12Device) -> Result<ID3D12RootSignature, GpuError> {
        unsafe {
            let ranges = [
                // SRV (source level)
                D3D12_DESCRIPTOR_RANGE {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
                    NumDescriptors: 1,
                    BaseShaderRegister: 0,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: 0,
                },
                // UAV (destination level)
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
                // Root constants (4 u32: source_width, source_height, dest_width, dest_height)
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

    /// Create compute pipeline state for Hi-Z shader
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

    /// Upload zbuffer from CPU to GPU (level 0 of pyramid)
    ///
    /// # Arguments
    /// * `zbuffer` - Slice of f32 depth values (row-major, width×height)
    pub fn upload_zbuffer(&mut self, zbuffer: &[f32]) -> Result<(), GpuError> {
        let expected_size = (self.width * self.height) as usize;
        if zbuffer.len() != expected_size {
            return Err(GpuError::DeviceCreation(
                windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0057u32 as i32)),
            ));
        }

        unsafe {
            // Map upload buffer
            let mut mapped_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            self.zbuffer_upload
                .Map(0, None, Some(&mut mapped_ptr))
                .map_err(GpuError::DeviceCreation)?;

            // Copy zbuffer data row by row (respecting D3D12 row pitch alignment)
            let row_pitch = Self::aligned_row_pitch(self.width, std::mem::size_of::<f32>() as u32);
            for y in 0..self.height as usize {
                let src_offset = y * self.width as usize;
                let dst_offset = y * (row_pitch as usize / std::mem::size_of::<f32>());
                std::ptr::copy_nonoverlapping(
                    zbuffer.as_ptr().add(src_offset),
                    (mapped_ptr as *mut f32).add(dst_offset),
                    self.width as usize,
                );
            }

            self.zbuffer_upload.Unmap(0, None);

            // Reset command list for upload
            self.device
                .command_allocator()
                .reset()
                .map_err(GpuError::CommandAllocatorCreation)?;
            self.command_list
                .Reset(self.device.command_allocator().raw(), None)
                .map_err(GpuError::DeviceCreation)?;

            // Transition zbuffer texture to COPY_DEST
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: std::mem::ManuallyDrop::new(Some(self.zbuffer_texture.clone())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_COMMON,
                        StateAfter: D3D12_RESOURCE_STATE_COPY_DEST,
                    }),
                },
            };
            self.command_list.ResourceBarrier(&[barrier]);

            // Copy from upload buffer to texture
            let row_pitch = Self::aligned_row_pitch(self.width, std::mem::size_of::<f32>() as u32);
            let src_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::ManuallyDrop::new(Some(self.zbuffer_upload.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: 0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                            Width: self.width,
                            Height: self.height,
                            Depth: 1,
                            RowPitch: row_pitch,
                        },
                    },
                },
            };

            let dst_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::ManuallyDrop::new(Some(self.zbuffer_texture.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };

            self.command_list
                .CopyTextureRegion(&dst_location, 0, 0, 0, &src_location, None);

            // Transition zbuffer texture to NON_PIXEL_SHADER_RESOURCE (for compute shader SRV)
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: std::mem::ManuallyDrop::new(Some(self.zbuffer_texture.clone())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                        StateAfter: D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
                    }),
                },
            };
            self.command_list.ResourceBarrier(&[barrier]);

            // Execute command list
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

            // Wait for upload to complete
            self.wait_for_gpu()?;
        }

        Ok(())
    }

    /// Build the Hi-Z pyramid on GPU
    ///
    /// Dispatches compute shader for each level, building from level 0 upward
    pub fn build_pyramid(&mut self) -> Result<(), GpuError> {
        for level in 1..self.level_count {
            self.build_pyramid_level(level)?;
        }
        Ok(())
    }

    /// Build a single pyramid level using compute shader
    fn build_pyramid_level(&mut self, level: u32) -> Result<(), GpuError> {
        if level == 0 || level >= self.level_count {
            return Err(GpuError::DeviceCreation(
                windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0057u32 as i32)),
            ));
        }

        unsafe {
            // Calculate source and destination dimensions
            let src_scale = 1u32 << (level - 1);
            let dst_scale = 1u32 << level;
            let src_width = self.width.div_ceil(src_scale);
            let src_height = self.height.div_ceil(src_scale);
            let dst_width = self.width.div_ceil(dst_scale);
            let dst_height = self.height.div_ceil(dst_scale);

            // Reset command list
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

            // Set descriptor table (SRV from source level, UAV to dest level)
            let descriptor_offset = ((level - 1) * 2) as usize; // 2 descriptors per level
            let mut gpu_handle = self.descriptor_heap.GetGPUDescriptorHandleForHeapStart();
            gpu_handle.ptr += (descriptor_offset * self.descriptor_size as usize) as u64;
            self.command_list
                .SetComputeRootDescriptorTable(0, gpu_handle);

            // Set root constants
            let constants = [src_width, src_height, dst_width, dst_height];
            self.command_list.SetComputeRoot32BitConstants(
                1,
                4,
                constants.as_ptr() as *const std::ffi::c_void,
                0,
            );

            // Get destination resource (always pyramid_levels, never zbuffer)
            // Dispatch compute shader (8×8 thread groups)
            let thread_groups_x = (dst_width + 7) / 8;
            let thread_groups_y = (dst_height + 7) / 8;
            self.command_list
                .Dispatch(thread_groups_x, thread_groups_y, 1);

            // Execute and wait
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

            self.wait_for_gpu()?;
        }

        Ok(())
    }

    /// Download pyramid from GPU to CPU HiZBuffer
    ///
    /// # Arguments
    /// * `hiz_buffer` - Target HiZBuffer to populate with pyramid data
    pub fn download_pyramid(
        &mut self,
        mut write_level_data: impl FnMut(u32, &[f32]),
        mut mark_valid: impl FnMut(),
    ) -> Result<(), GpuError> {
        unsafe {
            // Reset command list for copy operations
            self.device
                .command_allocator()
                .reset()
                .map_err(GpuError::CommandAllocatorCreation)?;
            self.command_list
                .Reset(self.device.command_allocator().raw(), None)
                .map_err(GpuError::DeviceCreation)?;

            // Track offset in readback buffer
            let mut readback_offset = 0usize;

            // Copy level 0 (zbuffer texture) to readback buffer
            {
                // Transition zbuffer texture to COPY_SOURCE
                let barrier = D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::ManuallyDrop::new(Some(
                                    self.zbuffer_texture.clone(),
                                )),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
                                StateAfter: D3D12_RESOURCE_STATE_COPY_SOURCE,
                            },
                        ),
                    },
                };
                self.command_list.ResourceBarrier(&[barrier]);

                // Copy texture to readback buffer
                let src_location = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: std::mem::ManuallyDrop::new(Some(self.zbuffer_texture.clone())),
                    Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        SubresourceIndex: 0,
                    },
                };

                let row_pitch =
                    Self::aligned_row_pitch(self.width, std::mem::size_of::<f32>() as u32);
                let dst_location = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: std::mem::ManuallyDrop::new(Some(self.pyramid_readback.clone())),
                    Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                            Offset: readback_offset as u64,
                            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                                Format:
                                    windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                                Width: self.width,
                                Height: self.height,
                                Depth: 1,
                                RowPitch: row_pitch,
                            },
                        },
                    },
                };

                self.command_list
                    .CopyTextureRegion(&dst_location, 0, 0, 0, &src_location, None);

                // Advance offset by aligned row pitch × height
                readback_offset += row_pitch as usize * self.height as usize;

                // Transition back to NON_PIXEL_SHADER_RESOURCE for next frame
                let barrier = D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::ManuallyDrop::new(Some(
                                    self.zbuffer_texture.clone(),
                                )),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_COPY_SOURCE,
                                StateAfter: D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
                            },
                        ),
                    },
                };
                self.command_list.ResourceBarrier(&[barrier]);
            }

            // Copy pyramid levels 1..N to readback buffer
            for (idx, level_texture) in self.pyramid_levels.iter().enumerate() {
                let level = (idx + 1) as u32;
                let scale = 1u32 << level;
                let level_width = self.width.div_ceil(scale);
                let level_height = self.height.div_ceil(scale);

                // Transition pyramid level to COPY_SOURCE
                let barrier = D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::ManuallyDrop::new(Some(level_texture.clone())),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
                                StateAfter: D3D12_RESOURCE_STATE_COPY_SOURCE,
                            },
                        ),
                    },
                };
                self.command_list.ResourceBarrier(&[barrier]);

                // Copy texture to readback buffer
                let src_location = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: std::mem::ManuallyDrop::new(Some(level_texture.clone())),
                    Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        SubresourceIndex: 0,
                    },
                };

                let row_pitch =
                    Self::aligned_row_pitch(level_width, std::mem::size_of::<f32>() as u32);
                let dst_location = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: std::mem::ManuallyDrop::new(Some(self.pyramid_readback.clone())),
                    Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                            Offset: readback_offset as u64,
                            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                                Format:
                                    windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_FLOAT,
                                Width: level_width,
                                Height: level_height,
                                Depth: 1,
                                RowPitch: row_pitch,
                            },
                        },
                    },
                };

                self.command_list
                    .CopyTextureRegion(&dst_location, 0, 0, 0, &src_location, None);

                // Advance offset by aligned row pitch × height
                readback_offset += row_pitch as usize * level_height as usize;

                // Transition back to UNORDERED_ACCESS for next frame
                let barrier = D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::ManuallyDrop::new(Some(level_texture.clone())),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_COPY_SOURCE,
                                StateAfter: D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
                            },
                        ),
                    },
                };
                self.command_list.ResourceBarrier(&[barrier]);
            }

            // Execute copy commands
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

            // Wait for copy to complete
            self.wait_for_gpu()?;

            // Map readback buffer and copy data to HiZBuffer
            let mut mapped_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            self.pyramid_readback
                .Map(0, None, Some(&mut mapped_ptr))
                .map_err(GpuError::DeviceCreation)?;

            let mut byte_offset = 0usize;

            // Copy level 0 (respecting aligned row pitch)
            {
                let row_pitch =
                    Self::aligned_row_pitch(self.width, std::mem::size_of::<f32>() as u32);
                let row_pitch_floats = row_pitch as usize / std::mem::size_of::<f32>();

                // Allocate temporary buffer for level data (contiguous, no padding)
                let mut level_data = vec![0.0f32; (self.width * self.height) as usize];

                // Copy row by row from aligned GPU buffer to contiguous CPU buffer
                for y in 0..self.height as usize {
                    let src_offset =
                        byte_offset / std::mem::size_of::<f32>() + y * row_pitch_floats;
                    let dst_offset = y * self.width as usize;
                    std::ptr::copy_nonoverlapping(
                        (mapped_ptr as *const f32).add(src_offset),
                        level_data.as_mut_ptr().add(dst_offset),
                        self.width as usize,
                    );
                }

                write_level_data(0, &level_data);
                byte_offset += self.height as usize * row_pitch as usize;
            }

            // Copy levels 1..N (respecting aligned row pitch)
            for level in 1..self.level_count {
                let scale = 1u32 << level;
                let level_width = self.width.div_ceil(scale);
                let level_height = self.height.div_ceil(scale);
                let row_pitch =
                    Self::aligned_row_pitch(level_width, std::mem::size_of::<f32>() as u32);
                let row_pitch_floats = row_pitch as usize / std::mem::size_of::<f32>();

                // Allocate temporary buffer for level data
                let mut level_data = vec![0.0f32; (level_width * level_height) as usize];

                // Copy row by row from aligned GPU buffer to contiguous CPU buffer
                for y in 0..level_height as usize {
                    let src_offset =
                        byte_offset / std::mem::size_of::<f32>() + y * row_pitch_floats;
                    let dst_offset = y * level_width as usize;
                    std::ptr::copy_nonoverlapping(
                        (mapped_ptr as *const f32).add(src_offset),
                        level_data.as_mut_ptr().add(dst_offset),
                        level_width as usize,
                    );
                }

                write_level_data(level, &level_data);
                byte_offset += level_height as usize * row_pitch as usize;
            }

            self.pyramid_readback.Unmap(0, None);

            // Mark pyramid as valid
            mark_valid();
        }

        Ok(())
    }

    /// Wait for GPU to finish executing commands
    fn wait_for_gpu(&mut self) -> Result<(), GpuError> {
        unsafe {
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
}
