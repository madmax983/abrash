//! Direct3D 12 device initialization and management
//!
//! This module provides wrappers for D3D12 device creation, command queue
//! management, and command allocator initialization for GPU compute binning.

#![cfg(feature = "gpu-binning")]

use std::fmt;
use windows::{
    Win32::Graphics::{Direct3D::D3D_FEATURE_LEVEL_11_0, Direct3D12::*, Dxgi::*},
    core::Result as WinResult,
};

/// GPU initialization and runtime errors
#[derive(Debug)]
pub enum GpuError {
    /// Failed to create DXGI factory
    FactoryCreation(windows::core::Error),
    /// Failed to enumerate GPU adapters
    AdapterEnumeration(windows::core::Error),
    /// No suitable GPU adapter found
    NoAdapter,
    /// Failed to create D3D12 device
    DeviceCreation(windows::core::Error),
    /// Failed to create command queue
    CommandQueueCreation(windows::core::Error),
    /// Failed to create command allocator
    CommandAllocatorCreation(windows::core::Error),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FactoryCreation(e) => write!(f, "Failed to create DXGI factory: {e}"),
            Self::AdapterEnumeration(e) => write!(f, "Failed to enumerate adapters: {e}"),
            Self::NoAdapter => write!(f, "No suitable GPU adapter found"),
            Self::DeviceCreation(e) => write!(f, "Failed to create D3D12 device: {e}"),
            Self::CommandQueueCreation(e) => write!(f, "Failed to create command queue: {e}"),
            Self::CommandAllocatorCreation(e) => {
                write!(f, "Failed to create command allocator: {e}")
            }
        }
    }
}

impl std::error::Error for GpuError {}

/// Wrapper for ID3D12CommandAllocator
pub struct D3D12CommandAllocator {
    allocator: ID3D12CommandAllocator,
}

impl D3D12CommandAllocator {
    /// Create a new command allocator
    fn new(device: &ID3D12Device, list_type: D3D12_COMMAND_LIST_TYPE) -> Result<Self, GpuError> {
        unsafe {
            let allocator: ID3D12CommandAllocator = device
                .CreateCommandAllocator(list_type)
                .map_err(GpuError::CommandAllocatorCreation)?;
            Ok(Self { allocator })
        }
    }

    /// Get the underlying ID3D12CommandAllocator
    #[must_use]
    pub fn raw(&self) -> &ID3D12CommandAllocator {
        &self.allocator
    }

    /// Reset the command allocator
    ///
    /// # Safety
    /// Must not be called while command lists allocated from this allocator are executing
    ///
    /// # Errors
    /// Returns a `windows::core::Error` if resetting the command allocator fails.
    pub unsafe fn reset(&self) -> WinResult<()> {
        unsafe { self.allocator.Reset() }
    }
}

/// Wrapper for ID3D12CommandQueue
pub struct D3D12CommandQueue {
    queue: ID3D12CommandQueue,
}

impl D3D12CommandQueue {
    /// Create a new command queue
    fn new(device: &ID3D12Device, desc: &D3D12_COMMAND_QUEUE_DESC) -> Result<Self, GpuError> {
        unsafe {
            let queue: ID3D12CommandQueue = device
                .CreateCommandQueue(desc)
                .map_err(GpuError::CommandQueueCreation)?;
            Ok(Self { queue })
        }
    }

    /// Get the underlying ID3D12CommandQueue
    #[must_use]
    pub fn raw(&self) -> &ID3D12CommandQueue {
        &self.queue
    }
}

/// Wrapper for ID3D12Device
pub struct D3D12Device {
    device: ID3D12Device,
    command_queue: D3D12CommandQueue,
    command_allocator: D3D12CommandAllocator,
}

impl D3D12Device {
    /// Create a new D3D12 device with command queue and allocator
    ///
    /// # Errors
    /// Returns `GpuError` if device creation, adapter enumeration, or command object creation fails
    pub fn new() -> Result<Self, GpuError> {
        unsafe {
            // Create DXGI factory
            let factory: IDXGIFactory4 = CreateDXGIFactory1().map_err(GpuError::FactoryCreation)?;

            // Enumerate adapters and find the first hardware adapter
            let adapter = Self::find_hardware_adapter(&factory)?;

            // Create D3D12 device with feature level 11.0
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device)
                .map_err(GpuError::DeviceCreation)?;

            let device = device.ok_or_else(|| {
                GpuError::DeviceCreation(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0057u32 as i32), // E_INVALIDARG
                ))
            })?;

            // Create command queue
            let queue_desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_COMPUTE,
                Priority: D3D12_COMMAND_QUEUE_PRIORITY_NORMAL.0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            };
            let command_queue = D3D12CommandQueue::new(&device, &queue_desc)?;

            // Create command allocator
            let command_allocator =
                D3D12CommandAllocator::new(&device, D3D12_COMMAND_LIST_TYPE_COMPUTE)?;

            Ok(Self {
                device,
                command_queue,
                command_allocator,
            })
        }
    }

    /// Find the first hardware adapter
    fn find_hardware_adapter(factory: &IDXGIFactory4) -> Result<IDXGIAdapter1, GpuError> {
        unsafe {
            for i in 0.. {
                let adapter = match factory.EnumAdapters1(i) {
                    Ok(adapter) => adapter,
                    Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => return Err(GpuError::NoAdapter),
                    Err(e) => return Err(GpuError::AdapterEnumeration(e)),
                };

                let desc = adapter.GetDesc1().map_err(GpuError::AdapterEnumeration)?;

                // Skip software adapters
                if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) == 0 {
                    return Ok(adapter);
                }
            }

            Err(GpuError::NoAdapter)
        }
    }

    /// Get the underlying ID3D12Device
    #[must_use]
    pub fn raw(&self) -> &ID3D12Device {
        &self.device
    }

    /// Get the command queue
    #[must_use]
    pub fn command_queue(&self) -> &D3D12CommandQueue {
        &self.command_queue
    }

    /// Get the command allocator
    #[must_use]
    pub fn command_allocator(&self) -> &D3D12CommandAllocator {
        &self.command_allocator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        // Test that we can create a D3D12 device
        let device = D3D12Device::new();
        assert!(device.is_ok(), "Failed to create D3D12 device");
    }

    #[test]
    fn test_device_has_command_queue() {
        let device = D3D12Device::new().expect("Failed to create device");
        let queue = device.command_queue();

        // Verify the queue is valid by checking we can get the raw pointer
        let _raw_queue = queue.raw();
    }

    #[test]
    fn test_device_has_command_allocator() {
        let device = D3D12Device::new().expect("Failed to create device");
        let allocator = device.command_allocator();

        // Verify the allocator is valid by checking we can get the raw pointer
        let _raw_allocator = allocator.raw();
    }

    #[test]
    fn test_command_allocator_reset() {
        let device = D3D12Device::new().expect("Failed to create device");
        let allocator = device.command_allocator();

        // Test that we can reset the allocator (when no command lists are executing)
        unsafe {
            let result = allocator.reset();
            assert!(result.is_ok(), "Failed to reset command allocator");
        }
    }

    #[test]
    fn test_gpu_error_display() {
        let err = GpuError::NoAdapter;
        assert_eq!(err.to_string(), "No suitable GPU adapter found");

        let err = GpuError::FactoryCreation(windows::core::Error::from_hresult(
            windows::core::HRESULT(0x8007_0057u32 as i32),
        ));
        assert!(err.to_string().contains("Failed to create DXGI factory"));
    }

    #[test]
    fn test_device_raw_access() {
        let device = D3D12Device::new().expect("Failed to create device");
        let _raw_device = device.raw();

        // If we get here without panicking, the raw device is valid
    }
}
