//! GPU compute binning implementation.
//!
//! This module adapts the `abrash-gpu` crate's binning capabilities to the
//! `PreparedTriangle` types used by `abrash::rasterizer::tile`.

#![cfg(feature = "gpu-binning")]

use crate::{
    hiz_buffer::{AABB3D, HiZBuffer},
    rasterizer::tile::PreparedTriangle,
};

pub use abrash_gpu::{
    BufferType, D3D12CommandAllocator, D3D12CommandQueue, D3D12Device, GpuBuffer, GpuError,
    TwoLevelBinningStats,
};

/// GPU compute binning pipeline wrapper that accepts engine-native triangle types.
pub struct GpuBinner {
    inner: abrash_gpu::GpuBinner,
    scratch: Vec<abrash_gpu::PreparedTriangleInput>,
}

impl GpuBinner {
    /// Create a new GPU binning pipeline.
    pub fn new(
        width: u32,
        height: u32,
        tile_size: u32,
        max_triangles: usize,
    ) -> Result<Self, GpuError> {
        Ok(Self {
            inner: abrash_gpu::GpuBinner::new(width, height, tile_size, max_triangles)?,
            scratch: Vec::new(),
        })
    }

    /// Bin triangles using GPU compute.
    pub fn bin_triangles(
        &mut self,
        triangles: &[PreparedTriangle],
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
    ) -> Result<(), GpuError> {
        self.sync_triangles(triangles);
        self.inner.bin_triangles(&self.scratch, heads, tails, nexts, tris)
    }

    /// Enable two-level hierarchical binning.
    pub fn enable_two_level_binning(&mut self) -> Result<(), GpuError> {
        self.inner.enable_two_level_binning()
    }

    /// Check whether two-level binning is enabled.
    #[must_use]
    pub fn is_two_level_enabled(&self) -> bool {
        self.inner.is_two_level_enabled()
    }

    /// Two-level binning: coarse pass, Hi-Z culling, then fine pass.
    pub fn bin_triangles_two_level(
        &mut self,
        triangles: &[PreparedTriangle],
        hiz_buffer: Option<&HiZBuffer>,
        heads: &mut [u32],
        tails: &mut [u32],
        nexts: &mut Vec<u32>,
        tris: &mut Vec<u32>,
    ) -> Result<TwoLevelBinningStats, GpuError> {
        self.sync_triangles(triangles);

        let adapter = hiz_buffer.map(HiZOcclusionAdapter);
        let trait_obj = adapter
            .as_ref()
            .map(|value| value as &dyn abrash_gpu::HiZOcclusion);

        self.inner
            .bin_triangles_two_level(&self.scratch, trait_obj, heads, tails, nexts, tris)
    }

    fn sync_triangles(&mut self, triangles: &[PreparedTriangle]) {
        self.scratch.clear();
        self.scratch
            .reserve(triangles.len().saturating_sub(self.scratch.capacity()));

        self.scratch.extend(
            triangles
                .iter()
                .map(|triangle| abrash_gpu::PreparedTriangleInput {
                    p0: abrash_gpu::ScreenVertexInput {
                        x: i32::from(triangle.p0.x),
                        y: i32::from(triangle.p0.y),
                        z: triangle.p0.z,
                    },
                    p1: abrash_gpu::ScreenVertexInput {
                        x: i32::from(triangle.p1.x),
                        y: i32::from(triangle.p1.y),
                        z: triangle.p1.z,
                    },
                    p2: abrash_gpu::ScreenVertexInput {
                        x: i32::from(triangle.p2.x),
                        y: i32::from(triangle.p2.y),
                        z: triangle.p2.z,
                    },
                    p0_fixed: abrash_gpu::VertexFixedInput {
                        x: i32::from(triangle.p0.x) << 8,
                        y: i32::from(triangle.p0.y) << 8,
                        z: (triangle.p0.z * 256.0) as i32,
                    },
                    p1_fixed: abrash_gpu::VertexFixedInput {
                        x: i32::from(triangle.p1.x) << 8,
                        y: i32::from(triangle.p1.y) << 8,
                        z: (triangle.p1.z * 256.0) as i32,
                    },
                    p2_fixed: abrash_gpu::VertexFixedInput {
                        x: i32::from(triangle.p2.x) << 8,
                        y: i32::from(triangle.p2.y) << 8,
                        z: (triangle.p2.z * 256.0) as i32,
                    },
                    dz_dx: triangle.dz_dx,
                    long_edge_is_left: triangle.long_edge_is_left,
                    color: triangle.color,
                    aabb_min_x: i32::from(triangle.aabb_min_x),
                    aabb_min_y: i32::from(triangle.aabb_min_y),
                    aabb_max_x: i32::from(triangle.aabb_max_x),
                    aabb_max_y: i32::from(triangle.aabb_max_y),
                    min_depth: triangle.min_depth,
                    max_depth: triangle.max_depth,
                }),
        );
    }
}

struct HiZOcclusionAdapter<'a>(&'a HiZBuffer);

impl abrash_gpu::HiZOcclusion for HiZOcclusionAdapter<'_> {
    fn is_coarse_bin_visible(&self, bin_aabb: abrash_gpu::Aabb3d) -> bool {
        self.0.is_coarse_bin_visible(AABB3D {
            min_x: bin_aabb.min_x,
            max_x: bin_aabb.max_x,
            min_y: bin_aabb.min_y,
            max_y: bin_aabb.max_y,
            min_depth: bin_aabb.min_depth,
            max_depth: bin_aabb.max_depth,
        })
    }
}
