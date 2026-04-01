//! GPU buffer management for compute binning.
//!
//! This module provides D3D12 buffer allocation and memory management for the GPU tile binning
//! pipeline. It handles three buffer types:
//!
//! - **Upload buffers**: CPU→GPU transfer (PreparedTriangle array)
//! - **UAV buffers**: GPU-writable storage (TileBin results)
//! - **Readback buffers**: GPU→CPU transfer (binning results)

#[cfg(feature = "gpu-binning")]
use windows::{
    Win32::Foundation::{E_FAIL, E_INVALIDARG},
    Win32::Graphics::{Direct3D12::*, Dxgi::Common::*},
    core::{Error, Result},
};

/// Buffer heap type determines access pattern and memory location.
#[cfg(feature = "gpu-binning")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferType {
    /// CPU_WRITE_COMBINED: Host writes, GPU reads. Used for uploading triangle data.
    Upload,
    /// DEFAULT heap with UAV flag: GPU read/write. Used for compute shader output.
    Uav,
    /// CPU_READ: GPU writes, host reads. Used for reading back binning results.
    Readback,
}

/// Wrapper for ID3D12Resource with heap type tracking.
#[cfg(feature = "gpu-binning")]
pub struct GpuBuffer {
    resource: ID3D12Resource,
    heap_type: BufferType,
    size: usize,
    mapped_ptr: Option<*mut u8>,
}

#[cfg(feature = "gpu-binning")]
impl GpuBuffer {
    /// Create a new GPU buffer with the specified heap type and size.
    ///
    /// # Safety
    ///
    /// The device must be valid and the size must not exceed available GPU memory.
    ///
    /// # Errors
    ///
    /// Returns a `windows::core::Error` if D3D12 resource creation fails.
    pub unsafe fn new(device: &ID3D12Device, buffer_type: BufferType, size: usize) -> Result<Self> {
        let heap_properties = match buffer_type {
            BufferType::Upload => D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_UPLOAD,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 1,
                VisibleNodeMask: 1,
            },
            BufferType::Uav => D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 1,
                VisibleNodeMask: 1,
            },
            BufferType::Readback => D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_READBACK,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 1,
                VisibleNodeMask: 1,
            },
        };

        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: 0,
            Width: size as u64,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: if buffer_type == BufferType::Uav {
                D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS
            } else {
                D3D12_RESOURCE_FLAG_NONE
            },
        };

        let initial_state = match buffer_type {
            BufferType::Upload => D3D12_RESOURCE_STATE_GENERIC_READ,
            BufferType::Uav => D3D12_RESOURCE_STATE_COMMON,
            BufferType::Readback => D3D12_RESOURCE_STATE_COPY_DEST,
        };

        let mut resource: Option<ID3D12Resource> = None;
        unsafe {
            device.CreateCommittedResource(
                &heap_properties,
                D3D12_HEAP_FLAG_NONE,
                &resource_desc,
                initial_state,
                None,
                &mut resource,
            )?
        };

        Ok(Self {
            resource: resource.ok_or_else(|| Error::from_hresult(E_FAIL))?,
            heap_type: buffer_type,
            size,
            mapped_ptr: None,
        })
    }

    /// Map the buffer for CPU access. Only valid for Upload and Readback buffers.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization (no GPU access during mapping).
    /// The returned pointer is valid until `unmap()` is called.
    ///
    /// # Errors
    ///
    /// Returns a `windows::core::Error` if the resource fails to map, such as if it is already mapped or is a UAV buffer.
    pub unsafe fn map(&mut self) -> Result<*mut u8> {
        if self.heap_type == BufferType::Uav {
            return Err(Error::from_hresult(E_INVALIDARG));
        }

        if self.mapped_ptr.is_some() {
            return Err(Error::from_hresult(E_FAIL)); // Already mapped
        }

        let mut ptr = std::ptr::null_mut();
        unsafe { self.resource.Map(0, None, Some(&mut ptr))? };

        self.mapped_ptr = Some(ptr.cast());
        Ok(ptr.cast())
    }

    /// Unmap the buffer.
    ///
    /// # Safety
    ///
    /// The caller must not use the pointer returned by `map()` after calling this.
    pub unsafe fn unmap(&mut self) {
        if self.mapped_ptr.is_some() {
            unsafe { self.resource.Unmap(0, None) };
            self.mapped_ptr = None;
        }
    }

    /// Get the underlying D3D12 resource.
    pub fn resource(&self) -> &ID3D12Resource {
        &self.resource
    }

    /// Get the buffer size in bytes.
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Get the buffer heap type.
    pub const fn buffer_type(&self) -> BufferType {
        self.heap_type
    }
}

#[cfg(feature = "gpu-binning")]
impl Drop for GpuBuffer {
    fn drop(&mut self) {
        unsafe {
            self.unmap();
        }
    }
}

// Tests are only compiled when gpu-binning feature is enabled
#[cfg(all(test, feature = "gpu-binning"))]
mod tests {
    use super::*;

    // Helper to create a test device
    unsafe fn create_test_device() -> Result<ID3D12Device> {
        let mut device: Option<ID3D12Device> = None;
        unsafe {
            D3D12CreateDevice(
                None,
                windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0,
                &mut device,
            )?
        };
        device.ok_or_else(|| Error::from_hresult(E_FAIL))
    }

    #[test]
    fn test_create_upload_buffer() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 80_000; // 80KB for 1000 triangles

            let buffer = GpuBuffer::new(&device, BufferType::Upload, size);
            assert!(buffer.is_ok(), "Upload buffer creation should succeed");

            let buffer = buffer.unwrap();
            assert_eq!(buffer.size(), size);
            assert_eq!(buffer.buffer_type(), BufferType::Upload);
        }
    }

    #[test]
    fn test_create_uav_buffer() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 8_388_480; // ~8MB for 8160 tiles (1028 bytes each)

            let buffer = GpuBuffer::new(&device, BufferType::Uav, size);
            assert!(buffer.is_ok(), "UAV buffer creation should succeed");

            let buffer = buffer.unwrap();
            assert_eq!(buffer.size(), size);
            assert_eq!(buffer.buffer_type(), BufferType::Uav);
        }
    }

    #[test]
    fn test_create_readback_buffer() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 8_388_480;

            let buffer = GpuBuffer::new(&device, BufferType::Readback, size);
            assert!(buffer.is_ok(), "Readback buffer creation should succeed");

            let buffer = buffer.unwrap();
            assert_eq!(buffer.size(), size);
            assert_eq!(buffer.buffer_type(), BufferType::Readback);
        }
    }

    #[test]
    fn test_map_upload_buffer() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 1024;

            let mut buffer = GpuBuffer::new(&device, BufferType::Upload, size).unwrap();

            let ptr = buffer.map().expect("Mapping upload buffer should succeed");
            assert!(!ptr.is_null(), "Mapped pointer should not be null");

            // Write some test data
            std::ptr::write_bytes(ptr, 0xAB, size);

            buffer.unmap();
        }
    }

    #[test]
    fn test_map_readback_buffer() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 1024;

            let mut buffer = GpuBuffer::new(&device, BufferType::Readback, size).unwrap();

            let ptr = buffer.map();
            assert!(ptr.is_ok(), "Mapping readback buffer should succeed");
            assert!(!ptr.unwrap().is_null(), "Mapped pointer should not be null");

            buffer.unmap();
        }
    }

    #[test]
    fn test_map_uav_buffer_fails() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 1024;

            let mut buffer = GpuBuffer::new(&device, BufferType::Uav, size).unwrap();

            let result = buffer.map();
            assert!(result.is_err(), "Mapping UAV buffer should fail");
        }
    }

    #[test]
    fn test_double_map_fails() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 1024;

            let mut buffer = GpuBuffer::new(&device, BufferType::Upload, size).unwrap();

            let first_map = buffer.map();
            assert!(first_map.is_ok(), "First map should succeed");

            let second_map = buffer.map();
            assert!(second_map.is_err(), "Second map without unmap should fail");

            buffer.unmap();
        }
    }

    #[test]
    fn test_buffer_cleanup() {
        unsafe {
            let device = create_test_device().expect("Failed to create D3D12 device");
            let size = 1024;

            {
                let mut buffer = GpuBuffer::new(&device, BufferType::Upload, size).unwrap();
                buffer.map().unwrap();
                // Buffer should auto-unmap on drop
            }
            // No crash = successful cleanup
        }
    }
}
