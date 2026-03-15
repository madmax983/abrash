#![allow(clippy::collapsible_if)]
//! Tile-based rendering for improved cache locality at high resolutions.
//!
//! This module implements a tile-based rasterizer that subdivides the framebuffer into 32×32 pixel
//! tiles and processes each tile independently. The working set (4KB pixels + 4KB depth = 8KB)
//! fits comfortably in L1 cache (typically 32-64 KB per core), providing significant performance
//! improvements when rendering large framebuffers that exceed L3 cache capacity.
//!
//! # Performance Characteristics
//!
//! Benchmark results show that tile-based rendering excels at **high resolutions with low triangle counts**:
//!
//! | Resolution | Framebuffer Size | Triangle Count | Performance vs Scanline |
//! |------------|-----------------|----------------|-------------------------|
//! | 800×600 | 3.84 MB | 10-500 | **1.3-3× slower** (use scanline) |
//! | 1920×1080 | 16.6 MB | ≤10 | **8% faster** ✓ |
//! | 1920×1080 | 16.6 MB | 200 | ~Even |
//! | 3840×2160 | 66.4 MB | 10 | **27% faster** ✓ |
//! | 3840×2160 | 66.4 MB | 50 | **6% faster** ✓ |
//! | 3840×2160 | 66.4 MB | ≥200 | **6% slower** (use scanline) |
//!
//! **Usage Guideline**: Use [`TileRenderer`] when resolution ≥ 1920×1080 AND triangle count ≤ 100.
//! Use [`fill_triangle_3d`](super::fill_triangle_3d) otherwise.
//!
//! # Parallel Rendering
//!
//! Enable the `parallel` feature for multi-threaded tile dispatch using Rayon:
//!
//! ```toml
//! [dependencies]
//! abrash = { version = "0.1", features = ["parallel"] }
//! ```
//!
//! With parallel rendering enabled, tiles are processed concurrently across all CPU cores, providing
//! near-linear speedup (3-4× on 4-core, 7-8× on 8-core systems). Each tile renders independently
//! into thread-local buffers, then merges into non-overlapping framebuffer regions safely.
//!
//! **Performance**: Expect 70-90% parallel efficiency for workloads with 100+ tiles (≥1920×1080).
//!
//! # Why the Crossover?
//!
//! - **At 800×600**: The 3.84 MB framebuffer fits in L2/L3 cache, so scanline doesn't suffer cache
//!   misses. Tiling overhead (prepare, bin, merge) dominates and makes it slower.
//! - **At 1920×1080**: The 16.6 MB framebuffer exceeds typical L3 cache (8-16 MB), causing cache
//!   thrashing in scanline. Tiled wins at low triangle counts where setup cost is minimal.
//! - **At 3840×2160**: The 66.4 MB framebuffer severely exceeds L3 cache. Scanline suffers massive
//!   cache thrashing while tiled's 8KB working set stays in L1. Tiled wins decisively at ≤50 triangles.
//!
//! # Example
//!
//! ```no_run
//! use abrash::rasterizer::{TileRenderer, ClipTriangle};
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Vec3;
//!
//! let mut fb = Framebuffer::new(3840, 2160).unwrap(); // 4K resolution
//! let mut zb = ZBuffer::new(3840, 2160).unwrap();
//! let mut renderer = TileRenderer::new(3840, 2160);
//!
//! let triangles: Vec<ClipTriangle> = vec![
//!     ((Vec3::new(-0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.0, 0.5, 0.5), 1.0),
//!      0xFF0000FF), // Red triangle
//! ];
//!
//! renderer.render_batch(&mut fb, &mut zb, &triangles);
//! ```

pub mod context;
pub mod renderer;
pub mod types;

pub use context::*;
pub use renderer::*;
pub use types::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use super::gouraud::draw_scanline_gouraud_simd_fast;
use super::gouraud::{GouraudEdgeWalker, GouraudGradients};
use super::texture::{draw_span_bilinear, draw_span_nearest, draw_span_trilinear};
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use super::texture::{draw_span_bilinear_simd, draw_span_nearest_simd, draw_span_trilinear_simd};
use super::{
    EdgeWalker, PerspectiveSpanStart, PerspectiveTextureEdgeWalker, PerspectiveTextureGradients,
    RECIPROCAL_TABLE, is_backface, sort_by_y,
};

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::hiz_buffer::{AABB3D, HiZBuffer};
use crate::math::{ScreenPoint, Vec2, Vec3, project_triangle_to_screen};
use crate::texture::{FilterMode, Texture};
use crate::zbuffer::ZBuffer;
use std::ops::{Deref, DerefMut};

/// A clip-space triangle with three vertices `(position, w)` and a flat color.
pub type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// A clip-space triangle with three vertices `(position, w)` and UV coordinates.
pub type TexturedClipTriangle = ((Vec3, f32), Vec2, (Vec3, f32), Vec2, (Vec3, f32), Vec2);

/// Helper function to compute the minimum and maximum Y bounds for clearing a tile,
/// based on the triangles intersecting it, and clears the specified tile regions.
use std::mem::MaybeUninit;

/// Render a single tile: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]

/// Render a triangle into tile-local buffers. Free function to avoid `&mut self` borrow conflicts.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_triangle_in_tile(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
) {
    let p0_y = tri.p0.y;
    let p1_y = tri.p1.y;
    let p2_y = tri.p2.y;

    let y_start = p0_y.max(tile_y0);
    let y_end = p2_y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    // Reconstruct ScreenPoint for EdgeWalker (inv_w unused for flat shading)
    let p0 = ScreenPoint {
        x: tri.p0.x,
        y: p0_y,
        z: tri.p0.z,
        inv_w: 1.0,
    };
    let p1 = ScreenPoint {
        x: tri.p1.x,
        y: p1_y,
        z: tri.p1.z,
        inv_w: 1.0,
    };
    let p2 = ScreenPoint {
        x: tri.p2.x,
        y: p2_y,
        z: tri.p2.z,
        inv_w: 1.0,
    };

    // Edge A: always p0→p2 (long edge)
    let mut edge_a = EdgeWalker::new(p0, p2);
    if y_start > p0_y {
        edge_a.step_n(i64::from(y_start) - i64::from(p0_y));
    }

    // Edge B: depends on whether y_start is above or below p1.y
    let mut edge_b = if y_start < p1_y {
        let mut e = EdgeWalker::new(p0, p1);
        if y_start > p0_y {
            e.step_n(i64::from(y_start) - i64::from(p0_y));
        }
        e
    } else {
        let mut e = EdgeWalker::new(p1, p2);
        if y_start > p1_y {
            e.step_n(i64::from(y_start) - i64::from(p1_y));
        }
        e
    };

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    let dz_dx = tri.dz_dx;
    let color = tri.color;

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    for y in y_start..=y_end {
        if y == p1_y && y != p0_y {
            edge_b = EdgeWalker::new(p1, p2);
        }

        let (x_start, x_end, z_left) = if tri.long_edge_is_left {
            ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z)
        } else {
            ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z)
        };

        process_tile_scanline_flat(&mut ctx, y, x_start, x_end, z_left, dz_dx, color);

        edge_a.step();
        edge_b.step();
    }
}

/// Render a single tile textured: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]

/// Render a textured triangle into tile-local buffers.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_triangle_in_tile_textured(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedTexturedTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
    texture: &Texture,
) {
    let y_start = tri.p0.y.max(tile_y0);
    let y_end = tri.p2.y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    let mut edge_a = PerspectiveTextureEdgeWalker::new(
        tri.p0,
        tri.p2,
        tri.p0.inv_w,
        tri.p2.inv_w,
        tri.u0,
        tri.u2,
        tri.v0,
        tri.v2,
    );
    if y_start > tri.p0.y {
        edge_a.step_n(i64::from(y_start) - i64::from(tri.p0.y));
    }

    let mut edge_b = if y_start < tri.p1.y {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p0,
            tri.p1,
            tri.p0.inv_w,
            tri.p1.inv_w,
            tri.u0,
            tri.u1,
            tri.v0,
            tri.v1,
        );
        if y_start > tri.p0.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p0.y));
        }
        e
    } else {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p1,
            tri.p2,
            tri.p1.inv_w,
            tri.p2.inv_w,
            tri.u1,
            tri.u2,
            tri.v1,
            tri.v2,
        );
        if y_start > tri.p1.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p1.y));
        }
        e
    };

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    for y in y_start..=y_end {
        if y == tri.p1.y && y != tri.p0.y {
            edge_b = PerspectiveTextureEdgeWalker::new(
                tri.p1,
                tri.p2,
                tri.p1.inv_w,
                tri.p2.inv_w,
                tri.u1,
                tri.u2,
                tri.v1,
                tri.v2,
            );
        }

        let (x_start, x_end, z_left, q_left, u_left, v_left) = if tri.long_edge_is_left {
            (
                (edge_a.x >> 16) as i32,
                (edge_b.x >> 16) as i32,
                edge_a.z,
                edge_a.q,
                edge_a.u,
                edge_a.v,
            )
        } else {
            (
                (edge_b.x >> 16) as i32,
                (edge_a.x >> 16) as i32,
                edge_b.z,
                edge_b.q,
                edge_b.u,
                edge_b.v,
            )
        };

        process_tile_scanline_textured(
            &mut ctx, y, x_start, x_end, z_left, q_left, u_left, v_left, tri, texture,
        );

        edge_a.step();
        edge_b.step();
    }
}

#[inline(always)]

fn process_tile_scanline_flat(
    ctx: &mut TileContext,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_left: f32,
    dz_dx: f32,
    color: u32,
) {
    let dx = i64::from(x_end) - i64::from(x_start);

    if dx <= 0 {
        // Single-pixel scanline
        if x_start >= ctx.x0 && x_start < ctx.x1 && x_start >= 0 && x_start <= ctx.screen_x_max {
            let tile_idx = ctx.get_indices(x_start, y);
            if z_left < ctx.depths[tile_idx] {
                ctx.depths[tile_idx] = z_left;
                ctx.pixels[tile_idx] = color;
            }
        }
    } else {
        // Clamp X to tile and screen bounds
        let xs = x_start.max(ctx.x0).max(0);
        let xe = x_end.min(ctx.x1 - 1).min(ctx.screen_x_max);

        if xs <= xe {
            // Calculate z at xs
            let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
            let z_at_xs = z_left + dx_start * dz_dx;

            let row_offset = ctx.get_row_offset(y);
            let col_start = (xs - ctx.x0) as usize;
            let col_end = (xe - ctx.x0) as usize;

            let pixels = &mut ctx.pixels[row_offset + col_start..=row_offset + col_end];
            let depths = &mut ctx.depths[row_offset + col_start..=row_offset + col_end];

            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            {
                if pixels.len() >= 8 {
                    rasterize_scanline_simd(pixels, depths, z_at_xs, dz_dx, color);
                } else {
                    rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
                }
            }
            #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
            rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
        }
    }
}

#[inline(always)]
fn process_tile_scanline_textured(
    ctx: &mut TileContext,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_left: f32,
    q_left: f32,
    u_left: f32,
    v_left: f32,
    tri: &PreparedTexturedTriangle,
    texture: &Texture,
) {
    let dx = i64::from(x_end) - i64::from(x_start);

    if dx <= 0 {
        // Single-pixel scanline
        if x_start >= ctx.x0 && x_start < ctx.x1 && x_start >= 0 && x_start <= ctx.screen_x_max {
            let tile_idx = ctx.get_indices(x_start, y);
            if z_left < ctx.depths[tile_idx] && q_left.abs() > 0.000_001 {
                ctx.depths[tile_idx] = z_left;
                let w = 1.0 / q_left;
                let u_tex = u_left * w;
                let v_tex = v_left * w;
                ctx.pixels[tile_idx] = match texture.filter_mode {
                    FilterMode::Nearest => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                    FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                    FilterMode::Trilinear => {
                        let w = 1.0 / q_left;
                        let w_sq = w * w;

                        let du_tex_dx =
                            (tri.gradients.du_dx * q_left - u_left * tri.gradients.dq_dx) * w_sq;
                        let dv_tex_dx =
                            (tri.gradients.dv_dx * q_left - v_left * tri.gradients.dq_dx) * w_sq;
                        let du_tex_dy =
                            (tri.gradients.du_dy * q_left - u_left * tri.gradients.dq_dy) * w_sq;
                        let dv_tex_dy =
                            (tri.gradients.dv_dy * q_left - v_left * tri.gradients.dq_dy) * w_sq;

                        let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                            .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                        let lod = 0.5 * max_rho_sq.log2();
                        texture.get_pixel_trilinear(u_tex, v_tex, lod)
                    }
                };
            }
        }
    } else {
        // Clamp X to tile and screen bounds
        let xs = x_start.max(ctx.x0).max(0);
        let xe = x_end.min(ctx.x1 - 1).min(ctx.screen_x_max);

        if xs <= xe {
            let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
            let z_start = z_left + dx_start * tri.gradients.dz_dx;
            let q_start = q_left + dx_start * tri.gradients.dq_dx;
            let u_start = u_left + dx_start * tri.gradients.du_dx;
            let v_start = v_left + dx_start * tri.gradients.dv_dx;

            let start = PerspectiveSpanStart {
                z: z_start,
                q: q_start,
                u: u_start,
                v: v_start,
            };

            let row_offset = ctx.get_row_offset(y);
            let col_start = (xs - ctx.x0) as usize;
            let col_end = (xe - ctx.x0) as usize;

            let pixels = &mut ctx.pixels[row_offset + col_start..=row_offset + col_end];
            let depths = &mut ctx.depths[row_offset + col_start..=row_offset + col_end];

            rasterize_scanline_textured(pixels, depths, texture, start, &tri.gradients);
        }
    }
}

fn rasterize_scanline_textured(
    pixels: &mut [u32],
    depths: &mut [f32],
    texture: &Texture,
    start: PerspectiveSpanStart,
    gradients: &PerspectiveTextureGradients,
) {
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;

    let len = pixels.len();
    let span_size = 16;
    let mut i = 0;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while i < len {
        let count = (len - i).min(span_size);

        // End values at 'i + count'
        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        let inv_count = RECIPROCAL_TABLE[count];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        let pixels_slice = &mut pixels[i..i + count];
        let depths_slice = &mut depths[i..i + count];

        match texture.filter_mode {
            FilterMode::Nearest => {
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_nearest_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_nearest(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_nearest(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Bilinear => {
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_bilinear_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_bilinear(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_bilinear(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Trilinear => {
                let w = w_start; // 1/q
                let w_sq = w * w;

                let du_tex_dx = (gradients.du_dx * q - u * gradients.dq_dx) * w_sq;
                let dv_tex_dx = (gradients.dv_dx * q - v * gradients.dq_dx) * w_sq;
                let du_tex_dy = (gradients.du_dy * q - u * gradients.dq_dy) * w_sq;
                let dv_tex_dy = (gradients.dv_dy * q - v * gradients.dq_dy) * w_sq;

                let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                    .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                let lod = 0.5 * max_rho_sq.log2();

                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_trilinear_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                            lod,
                        );
                    }
                } else {
                    draw_span_trilinear(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                        lod,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_trilinear(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                    lod,
                );
            }
        }

        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;

        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;
        i += count;
    }
}

/// Scalar scanline rasterization: process 1 pixel per iteration
#[inline(always)]

/// AVX2 vectorized scanline rasterization: process 8 pixels per iteration
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[inline(always)]
#[allow(dead_code)]
fn rasterize_scanline_simd(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_at_xs: f32,
    dz_dx: f32,
    color: u32,
) {
    use std::arch::x86_64::{
        __m256i, _CMP_LT_OQ, _mm256_add_ps, _mm256_blendv_ps, _mm256_castps_si256,
        _mm256_castsi256_ps, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_loadu_si256,
        _mm256_movemask_ps, _mm256_mul_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
        _mm256_storeu_ps, _mm256_storeu_si256,
    };

    let len = pixels.len();
    let mut i = 0;

    // --- Optimization Idea 1: Align the loop ---
    // Handle the first few pixels (0-7) with scalar code until the pointer is 32-byte aligned.
    // AVX2 loads/stores are faster when aligned to 32 bytes (256 bits).
    // The depths buffer is allocated via AlignedBuffer so it's aligned, but `pixels`
    // is a slice into that buffer, so it might start at an unaligned offset depending on x_start.
    //
    // Actually, `TileRenderer` slices `tile_pixels` based on `x - tile_x0`. Since `tile_x0`
    // is always a multiple of 32, and `AlignedBuffer` is 32-byte aligned, the offset depends on `x`.
    // We align based on the destination address of `pixels` (color buffer).
    // Depths and pixels have the same alignment offset relative to 32 bytes because they are
    // both accessed with the same index `i`.

    let align_mask = 0x1F; // 32 bytes - 1
    let addr = pixels.as_ptr() as usize;
    let misalign = addr & align_mask;
    let pre_simd_count = if misalign == 0 {
        0
    } else {
        (32 - misalign) / 4 // 4 bytes per pixel
    };

    // Ensure we don't overrun the buffer if it's very small
    let pre_simd_count = pre_simd_count.min(len);

    let mut z = z_at_xs;

    // Process initial unaligned pixels
    for k in 0..pre_simd_count {
        unsafe {
            let d = depths.get_unchecked_mut(k);
            if z < *d {
                *d = z;
                *pixels.get_unchecked_mut(k) = color;
            }
        }
        z += dz_dx;
    }

    i += pre_simd_count;

    // --- Main SIMD Loop (Aligned) ---
    unsafe {
        use std::arch::x86_64::{_CMP_GE_OQ, _mm256_cmp_ps, _mm256_store_ps, _mm256_store_si256};

        // Setup: stride vector for incrementing depths by 8*dz_dx per iteration
        let stride_vec = _mm256_set1_ps(8.0 * dz_dx);

        // Initialize depth vector using vector arithmetic:
        // depths = z (current) + [0, 1, 2, 3, 4, 5, 6, 7] * dz_dx
        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let dz_vec = _mm256_set1_ps(dz_dx);
        let base = _mm256_set1_ps(z);
        let mut depths_vec = _mm256_add_ps(base, _mm256_mul_ps(offsets, dz_vec));

        let color_vec = _mm256_set1_epi32(color as i32);

        // Process 8 pixels at a time with AVX2
        while i + 8 <= len {
            // Load zbuffer values for 8 pixels
            // If we aligned correctly, this should be an aligned load for `pixels`.
            // However, `depths` buffer is separate. `AlignedBuffer` ensures start is aligned.
            // Since we advanced `i` to align `pixels`, `depths` at `i` is also aligned
            // ONLY IF `tile_pixels` and `tile_depths` had same initial alignment modulo 32.
            // `AlignedBuffer::new` ensures 32-byte alignment for start.
            // Since we index both by the same `i` (relative to start of slice), and slices start
            // at same offset relative to aligned base (same x_start), they are both aligned.
            //
            // Use aligned load/store intrinsics where possible.
            // Note: `loadu` is still safe and fast on Haswell+ even if aligned.
            // `store` (aligned) traps if unaligned, so we must be sure.
            // We aligned based on `pixels`. `depths` should match.

            let zb_ptr = depths.as_mut_ptr().add(i);
            // We use loadu just to be safe in case of weird offsets, but stores will be aligned.
            // Actually, let's use loadu for reads to be robust, and aligned stores because we calculated alignment.
            let zb_vals = _mm256_loadu_ps(zb_ptr);

            // Optimization Idea 2: Early Out (Occlusion Culling)
            // Check if ALL pixels fail the depth test (depth >= zbuffer)
            // _CMP_GE_OQ: Greater-than or Equal (Ordered, Non-signaling)
            let ge_mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_GE_OQ);
            let ge_bits = _mm256_movemask_ps(ge_mask);

            if ge_bits == 0xFF {
                // All pixels occluded. Skip write.
            } else {
                // At least one pixel is visible.
                // Compare: depth < zbuffer
                let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);
                let mask_bits = _mm256_movemask_ps(mask);

                if mask_bits == 0xFF {
                    // Fast path: All pixels visible.
                    // Store depths and colors directly using Aligned Stores.
                    _mm256_store_ps(zb_ptr, depths_vec);

                    let pixels_ptr = pixels.as_mut_ptr().add(i) as *mut __m256i;
                    _mm256_store_si256(pixels_ptr, color_vec);
                } else {
                    // Partial write path
                    // 1. Update depths
                    let blended_depths = _mm256_blendv_ps(zb_vals, depths_vec, mask);
                    // Use aligned store since we are aligned
                    _mm256_store_ps(zb_ptr, blended_depths);

                    // 2. Update pixels
                    let pixels_ptr = pixels.as_mut_ptr().add(i) as *mut __m256i;
                    // Read old pixels (aligned load)
                    let old_pixels = _mm256_loadu_si256(pixels_ptr as *const __m256i);

                    let old_pixels_ps = _mm256_castsi256_ps(old_pixels);
                    let color_vec_ps = _mm256_castsi256_ps(color_vec);

                    let blended_pixels_ps = _mm256_blendv_ps(old_pixels_ps, color_vec_ps, mask);

                    // Aligned store
                    _mm256_store_si256(pixels_ptr, _mm256_castps_si256(blended_pixels_ps));
                }
            }

            // Increment depths by stride (8*dz_dx) for next iteration
            depths_vec = _mm256_add_ps(depths_vec, stride_vec);
            i += 8;
        }

        // Update the scalar z tracker for the tail loop
        // z corresponds to depth at 'i' (start of this iteration block)
        // But we incremented depths_vec already for the *next* block.
        // We need to sync the scalar 'z' to the current 'i'.
        // Actually, easiest is just to recalculate z from scratch or extract from vector.
        // Or just maintain 'z' mathematically.
        // We've processed `i` pixels (including pre-simd).
        // `z` variable currently holds value at start of SIMD loop.
        // We need z at `i` (current).
        // Since we didn't update scalar `z` inside SIMD loop, we do it now.
        // The SIMD loop ran (i - pre_simd_count) / 8 iterations.
        let simd_pixels = i - pre_simd_count;
        z += (simd_pixels as f32) * dz_dx;
    }

    // Handle remaining pixels with scalar fallback
    let mut z = z_at_xs + (i as f32) * dz_dx;
    for j in i..len {
        if z < depths[j] {
            depths[j] = z;
            pixels[j] = color;
        }
        z += dz_dx;
    }
}

/// Fallback for when SIMD is not available (non-x86_64 or feature disabled)
#[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
#[inline(always)]
#[allow(dead_code)]
fn rasterize_scanline_simd(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_at_xs: f32,
    dz_dx: f32,
    color: u32,
) {
    rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
}

pub(crate) fn render_triangle_in_tile_gouraud(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedGouraudTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
) {
    let p0_y = tri.p0.y;
    let p2_y = tri.p2.y;

    let y_start = p0_y.max(tile_y0);
    let y_end = p2_y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    // Edge Walking
    let p0 = tri.p0.to_screen_point(1.0);
    let p1 = tri.p1.to_screen_point(1.0);
    let p2 = tri.p2.to_screen_point(1.0);

    // Convert fixed point colors back to Vec3 for EdgeWalker initialization
    let c0 = Vec3::new(
        tri.c0.0 as f32 / 65536.0,
        tri.c0.1 as f32 / 65536.0,
        tri.c0.2 as f32 / 65536.0,
    );
    let c1 = Vec3::new(
        tri.c1.0 as f32 / 65536.0,
        tri.c1.1 as f32 / 65536.0,
        tri.c1.2 as f32 / 65536.0,
    );
    let c2 = Vec3::new(
        tri.c2.0 as f32 / 65536.0,
        tri.c2.1 as f32 / 65536.0,
        tri.c2.2 as f32 / 65536.0,
    );

    let mut edge_a = GouraudEdgeWalker::new(p0, p2, c0, c2);
    if y_start > p0.y {
        edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
    }

    let mut edge_b = if y_start < tri.p1.y {
        let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
        if y_start > p0.y {
            e.step_n(i64::from(y_start) - i64::from(p0.y));
        }
        e
    } else {
        let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
        if y_start > tri.p1.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p1.y));
        }
        e
    };

    let dz_dx = tri.gradients.dz_dx;
    let dc_dx = tri.gradients.dc_dx;

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    for y in y_start..=y_end {
        if y == tri.p1.y && y != p0_y {
            edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
        }

        let (x_start, x_end, z_left, c_left) = if tri.long_edge_is_left {
            (
                (edge_a.x >> 16) as i32,
                (edge_b.x >> 16) as i32,
                edge_a.z,
                edge_a.c,
            )
        } else {
            (
                (edge_b.x >> 16) as i32,
                (edge_a.x >> 16) as i32,
                edge_b.z,
                edge_b.c,
            )
        };

        process_tile_scanline_gouraud(&mut ctx, y, x_start, x_end, z_left, c_left, tri);

        edge_a.step();
        edge_b.step();
    }
}
