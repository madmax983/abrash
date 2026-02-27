//! Compatibility wrapper around the external `abrash-gpu` crate.

#![cfg(feature = "gpu-binning")]

pub use abrash_gpu::{
    BufferType, D3D12CommandAllocator, D3D12CommandQueue, D3D12Device, GpuBuffer, GpuError,
    TwoLevelBinningStats,
};
