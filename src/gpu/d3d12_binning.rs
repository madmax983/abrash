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
        System::Threading::{CreateEventW, WaitForSingleObject},
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
        let compute_pso = Self::create_compute_pso(device.raw(), &root_signature)?;

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
    ) -> Result<ID3D12PipelineState, GpuError> {
        let shader_bytecode = include_bytes!("../../shaders/bin_triangles.cso");

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
