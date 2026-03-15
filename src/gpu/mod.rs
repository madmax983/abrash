//! Compatibility wrapper around the external `abrash-gpu` crate.

#![cfg(feature = "gpu-binning")]

use crate::{
    hiz_buffer::{AABB3D, HiZBuffer},
    tile_renderer::PreparedTriangle,
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
        tile_bins: &mut Vec<Vec<usize>>,
    ) -> Result<(), GpuError> {
        self.sync_triangles(triangles);
        self.inner.bin_triangles(&self.scratch, tile_bins)
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
        tile_bins: &mut Vec<Vec<usize>>,
    ) -> Result<TwoLevelBinningStats, GpuError> {
        self.sync_triangles(triangles);

        let hiz_fn = hiz_buffer.map(|hiz| {
            move |bin_aabb: abrash_gpu::Aabb3d| {
                hiz.is_coarse_bin_visible(AABB3D {
                    min_x: bin_aabb.min_x,
                    max_x: bin_aabb.max_x,
                    min_y: bin_aabb.min_y,
                    max_y: bin_aabb.max_y,
                    min_depth: bin_aabb.min_depth,
                    max_depth: bin_aabb.max_depth,
                })
            }
        });

        self.inner
            .bin_triangles_two_level(&self.scratch, hiz_fn.as_ref(), tile_bins)
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

/// GPU Hi-Z pyramid builder wrapper that writes directly into `HiZBuffer`.
pub struct GpuHiZBuilder {
    inner: abrash_gpu::GpuHiZBuilder,
}

impl GpuHiZBuilder {
    /// Create a new GPU Hi-Z builder.
    pub fn new(width: u32, height: u32) -> Result<Self, GpuError> {
        Ok(Self {
            inner: abrash_gpu::GpuHiZBuilder::new(width, height)?,
        })
    }

    /// Upload a z-buffer to GPU memory.
    pub fn upload_zbuffer(&mut self, zbuffer: &[f32]) -> Result<(), GpuError> {
        self.inner.upload_zbuffer(zbuffer)
    }

    /// Build the Hi-Z pyramid on GPU.
    pub fn build_pyramid(&mut self) -> Result<(), GpuError> {
        self.inner.build_pyramid()
    }

    /// Download the GPU-built pyramid into `HiZBuffer`.
    pub fn download_pyramid(&mut self, hiz_buffer: &mut HiZBuffer) -> Result<(), GpuError> {
        let mut write_level_data =
            |level: u32, data: &[f32]| hiz_buffer.write_level_data(level, data);
        let mut mark_valid = || hiz_buffer.mark_valid();
        self.inner
            .download_pyramid(&mut write_level_data, &mut mark_valid)
    }
}
