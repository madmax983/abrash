//! GPU compute acceleration for triangle binning
//!
//! This module provides DirectX 12 GPU compute capabilities for accelerating
//! triangle binning operations in the tile renderer.

#![cfg(feature = "gpu-binning")]

mod buffers;
mod d3d12_binning;
mod d3d12_device;

pub use buffers::{BufferType, GpuBuffer};
pub use d3d12_binning::{GpuBinner, GpuHiZBuilder};
pub use d3d12_device::{D3D12CommandAllocator, D3D12CommandQueue, D3D12Device, GpuError};
