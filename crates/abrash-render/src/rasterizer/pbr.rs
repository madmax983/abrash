//! # The PBR Rasterizer: Forging Realism 🛡️
//!
//! In the age of myths, rendering was a simple art of Gouraud and Phong. Surfaces were plastic,
//! and light was a mere suggestion. Then came the **Physically Based Rendering (PBR)** revolution.
//!
//! This module implements the **Cook-Torrance Specular BRDF**, a model that treats light not as magic,
//! but as photons interacting with micro-facets on a surface. It brings:
//!
//! *   **Conservation of Energy**: Surfaces cannot reflect more light than they receive.
//! *   **Fresnel Effect**: Surfaces become more reflective at glancing angles.
//! *   **Micro-facet Theory**: Roughness simulates microscopic surface irregularities.
//!
//! ## The Hero's Journey: A Metallic Sphere 🌟
//!
//! Imagine a hero, a sphere of pure gold, placed in a dark void illuminated by a single holy light.
//!
//! ```
//! use abrash_core::framebuffer::Framebuffer;
//! use abrash_core::zbuffer::ZBuffer;
//! use abrash_core::math::Vec3;
//! use abrash_render::rasterizer::pbr::fill_triangle_pbr;
//!
//! // 1. The Canvas
//! let width = 100;
//! let height = 100;
//! let mut fb = Framebuffer::new(width, height).unwrap();
//! let mut zb = ZBuffer::new(width, height).unwrap();
//!
//! // 2. The Triangle (A shard of the sphere)
//! let v0 = ((Vec3::new(0.0, 5.0, 0.0), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));
//! let v1 = ((Vec3::new(-5.0, -5.0, 0.0), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(-1.0, -1.0, 0.0));
//! let v2 = ((Vec3::new(5.0, -5.0, 0.0), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, -1.0, 0.0));
//!
//! // 3. The Material (Gold)
//! let albedo = Vec3::new(1.00, 0.71, 0.29); // Gold color
//! let metallic = 1.0; // Pure metal
//! let roughness = 0.2; // Polished but not mirror-like
//! let ao = 1.0; // No occlusion
//!
//! // 4. The Light (A white sun)
//! let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
//! let light_color = Vec3::new(1.0, 1.0, 1.0);
//! let view_pos = Vec3::new(0.0, 0.0, 10.0);
//!
//! // 5. The Act of Creation
//! fill_triangle_pbr(
//!     &mut fb, &mut zb,
//!     v0, v1, v2,
//!     albedo, metallic, roughness, ao,
//!     light_dir, light_color, view_pos
//! );
//!
//! // The pixel at the center now holds the glint of gold.
//! let center_pixel = fb.get_pixel(50, 50).unwrap();
//! assert_ne!(center_pixel, 0xFF000000);
//! ```

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_triangle_to_screen};
use crate::rasterizer::core::{
    FIXED_SCALE, assert_same_dimensions, color_to_u32_scaled, is_backface, sort_by_y,
};
use crate::zbuffer::ZBuffer;
use std::f32::consts::PI;

/// Represents the physical properties of a surface material.
#[derive(Clone, Copy)]
pub struct PbrMaterial {
    /// The base color of the surface. For metals, this is the specular color.
    pub albedo: Vec3,
    /// How "metallic" the surface is (0.0 = Dielectric, 1.0 = Metal).
    pub metallic: f32,
    /// Surface microsurface irregularity (0.0 = Smooth, 1.0 = Rough).
    pub roughness: f32,
    /// Ambient Occlusion factor (0.0 = Occluded, 1.0 = Exposed).
    pub ao: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PbrConstants {
    a2: f32,
    a2_minus_1: f32,
    k: f32,
    one_minus_k: f32,
    f0: Vec3,
    #[allow(dead_code)]
    dielectric_f0: Vec3,
    neg_light_dir: Vec3,
    light_color: Vec3,
    view_pos: Vec3,
    albedo: Vec3,
    metallic: f32,
    ao: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PbrSpanStart {
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PbrGradients {
    dz_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
    dwx_dx: f32,
    dwy_dx: f32,
    dwz_dx: f32,
}

impl PbrGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        w0: Vec3,
        w1: Vec3,
        w2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let uwx = w1.x - w0.x;
        let uwy = w1.y - w0.y;
        let uwz = w1.z - w0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vwx = w2.x - w0.x;
        let vwy = w2.y - w0.y;
        let vwz = w2.z - w0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_wx = uy * vwx - uwx * vy;
        let dwx_dx = nx_wx * inv_nz;

        let nx_wy = uy * vwy - uwy * vy;
        let dwy_dx = nx_wy * inv_nz;

        let nx_wz = uy * vwz - uwz * vy;
        let dwz_dx = nx_wz * inv_nz;

        (
            Self {
                dz_dx,
                dnx_dx,
                dny_dx,
                dnz_dx,
                dwx_dx,
                dwy_dx,
                dwz_dx,
            },
            nz > 0.0,
        )
    }
}

struct PbrEdgeWalker {
    x: i64,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dwx_dy: f32,
    dwy_dy: f32,
    dwz_dy: f32,
}

impl PbrEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        n_start: Vec3,
        n_end: Vec3,
        w_start: Vec3,
        w_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dwx_dy = (w_end.x - w_start.x) * inv_h;
        let dwy_dy = (w_end.y - w_start.y) * inv_h;
        let dwz_dy = (w_end.z - w_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            wx: w_start.x,
            wy: w_start.y,
            wz: w_start.z,
            dx_dy,
            dz_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dwx_dy,
            dwy_dy,
            dwz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.wx += self.dwx_dy;
        self.wy += self.dwy_dy;
        self.wz += self.dwz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.wx += self.dwx_dy * n_f;
        self.wy += self.dwy_dy * n_f;
        self.wz += self.dwz_dy * n_f;
    }
}

// Optimized BRDF functions using precomputed constants

// Distribution GGX
// n_dot_h2: (n.dot(h))^2
fn distribution_ggx_optimized(n_dot_h2: f32, constants: &PbrConstants) -> f32 {
    let denom = n_dot_h2 * constants.a2_minus_1 + 1.0;
    let denom = PI * denom * denom;
    constants.a2 / denom.max(0.000_000_1)
}

// Geometry Schlick-GGX
fn geometry_schlick_ggx_optimized(n_dot_v: f32, constants: &PbrConstants) -> f32 {
    let nom = n_dot_v;
    let denom = n_dot_v * constants.one_minus_k + constants.k;
    nom / denom.max(0.000_000_1)
}

fn geometry_smith_optimized(n_dot_v: f32, n_dot_l: f32, constants: &PbrConstants) -> f32 {
    let ggx2 = geometry_schlick_ggx_optimized(n_dot_v, constants);
    let ggx1 = geometry_schlick_ggx_optimized(n_dot_l, constants);
    ggx1 * ggx2
}

// Fresnel Schlick
fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    // 5 muls
    let one_minus_cos = 1.0 - cos_theta;
    let pow2 = one_minus_cos * one_minus_cos;
    let pow4 = pow2 * pow2;
    let pow5 = pow4 * one_minus_cos;
    f0.lerp(Vec3::ONE, pow5)
}

/// Fills a 3D triangle using Physically Based Rendering (PBR).
///
/// This function rasterizes a triangle, interpolating normals and world positions per pixel,
/// and computing lighting using the Cook-Torrance BRDF.
///
/// # Arguments
///
/// *   `fb` - The framebuffer to draw to.
/// *   `zb` - The Z-buffer for depth testing.
/// *   `v0`, `v1`, `v2` - Vertices, each containing `((ClipPos, W), Normal, WorldPos)`.
/// *   `albedo` - Surface color.
/// *   `metallic` - Metallic factor (0.0 - 1.0).
/// *   `roughness` - Roughness factor (0.0 - 1.0).
/// *   `ao` - Ambient Occlusion factor (0.0 - 1.0).
/// *   `light_dir` - Direction *of* the light (not to light).
/// *   `light_color` - Radiance of the light.
/// *   `view_pos` - Position of the camera in world space.
///
/// # Performance
///
/// Uses precomputed constants for invariant BRDF terms (like $\alpha^2$ and $k$) to reduce per-pixel cost.
/// Uses fast approximations for Gamma Correction ($\gamma \approx 2.0$ via `sqrt`).
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_pbr(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    albedo: Vec3,
    metallic: f32,
    roughness: f32,
    ao: f32,
    light_dir: Vec3,
    light_color: Vec3,
    view_pos: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(
        v0,
        v1,
        v2,
        |v| v.0,
        |a, b, t| {
            (
                (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                a.1.lerp(b.1, t),
                a.2.lerp(b.2, t),
            )
        },
    );

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    // Precompute constants
    let a = roughness * roughness;
    let a2 = a * a;
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let dielectric_f0 = Vec3::new(0.04, 0.04, 0.04);
    let f0 = dielectric_f0 + (albedo - dielectric_f0) * metallic;

    let constants = PbrConstants {
        a2,
        a2_minus_1: a2 - 1.0,
        k,
        one_minus_k: 1.0 - k,
        f0,
        dielectric_f0,
        neg_light_dir: light_dir * -1.0,
        light_color,
        view_pos,
        albedo,
        metallic,
        ao,
    };

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped[base];
        let v1 = clipped[base + 1];
        let v2 = clipped[base + 2];

        // Project to screen
        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        let n0 = v0.1;
        let n1 = v1.1;
        let n2 = v2.1;
        let w0 = v0.2;
        let w1 = v1.2;
        let w2 = v2.2;

        let mut verts = [(p0_orig, n0, w0), (p1_orig, n1, w1), (p2_orig, n2, w2)];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, n0, w0), (p1, n1, w1), (p2, n2, w2)] = verts;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        let (gradients, long_edge_is_left) = PbrGradients::new(p0, p1, p2, n0, n1, n2, w0, w1, w2);

        let mut edge_a = PbrEdgeWalker::new(p0, p2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PbrEdgeWalker::new(p0, p1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PbrEdgeWalker::new(p1, p2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PbrEdgeWalker::new(p1, p2, n1, n2, w1, w2);
            }

            let (x_start, x_end, z_left, nx_left, ny_left, nz_left, wx_left, wy_left, wz_left) =
                if long_edge_is_left {
                    (
                        (edge_a.x >> 16) as i32,
                        (edge_b.x >> 16) as i32,
                        edge_a.z,
                        edge_a.nx,
                        edge_a.ny,
                        edge_a.nz,
                        edge_a.wx,
                        edge_a.wy,
                        edge_a.wz,
                    )
                } else {
                    (
                        (edge_b.x >> 16) as i32,
                        (edge_a.x >> 16) as i32,
                        edge_b.z,
                        edge_b.nx,
                        edge_b.ny,
                        edge_b.nz,
                        edge_b.wx,
                        edge_b.wy,
                        edge_b.wz,
                    )
                };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_pbr(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    PbrSpanStart {
                        z: z_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        wx: wx_left,
                        wy: wy_left,
                        wz: wz_left,
                    },
                    &gradients,
                    &constants,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_scanline_pbr(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PbrSpanStart,
    gradients: &PbrGradients,
    constants: &PbrConstants,
) {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    if (x_end - x_start + 1) >= 32
        && is_x86_feature_detected!("avx2")
        && is_x86_feature_detected!("fma")
    {
        unsafe {
            draw_scanline_pbr_simd(fb, zb, y, x_start, x_end, start, gradients, constants);
        }
        return;
    }

    draw_scanline_pbr_scalar(fb, zb, y, x_start, x_end, start, gradients, constants);
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[allow(clippy::too_many_arguments)]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn draw_scanline_pbr_simd(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PbrSpanStart,
    gradients: &PbrGradients,
    constants: &PbrConstants,
) {
    use std::arch::x86_64::{
        __m256i, _CMP_LT_OQ, _mm256_add_ps, _mm256_blendv_ps, _mm256_castps_si256,
        _mm256_castsi256_ps, _mm256_cmp_ps, _mm256_cvtps_epi32, _mm256_div_ps, _mm256_fmadd_ps,
        _mm256_loadu_ps, _mm256_loadu_si256, _mm256_max_ps, _mm256_movemask_ps, _mm256_mul_ps,
        _mm256_or_si256, _mm256_rsqrt_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
        _mm256_slli_epi32, _mm256_sqrt_ps, _mm256_storeu_ps, _mm256_storeu_si256, _mm256_sub_ps,
    };

    // Wrap entire body in unsafe block to satisfy unsafe_op_in_unsafe_fn lint
    // used by #[target_feature] intrinsics
    unsafe {
        if y < 0 || y >= fb.height() as i32 {
            return;
        }

        let width = fb.width() as i32;
        let mut xs = x_start;
        let mut xe = x_end;

        // Initial scalar advance to handle negative xs
        let mut z = start.z;
        let mut nx = start.nx;
        let mut ny = start.ny;
        let mut nz = start.nz;
        let mut wx = start.wx;
        let mut wy = start.wy;
        let mut wz = start.wz;

        if xs < 0 {
            let diff = -i64::from(xs);
            let diff_f = diff as f32;
            z += diff_f * gradients.dz_dx;
            nx += diff_f * gradients.dnx_dx;
            ny += diff_f * gradients.dny_dx;
            nz += diff_f * gradients.dnz_dx;
            wx += diff_f * gradients.dwx_dx;
            wy += diff_f * gradients.dwy_dx;
            wz += diff_f * gradients.dwz_dx;
            xs = 0;
        }

        if xe >= width {
            xe = width - 1;
        }

        if xs > xe {
            return;
        }

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let i = (y_offset + xs as usize) as usize;
        let end_i = (y_offset + xe as usize) as usize;
        let count = end_i - i + 1;

        // Vectors for gradients
        let dz_vec = _mm256_set1_ps(gradients.dz_dx);
        let dnx_vec = _mm256_set1_ps(gradients.dnx_dx);
        let dny_vec = _mm256_set1_ps(gradients.dny_dx);
        let dnz_vec = _mm256_set1_ps(gradients.dnz_dx);
        let dwx_vec = _mm256_set1_ps(gradients.dwx_dx);
        let dwy_vec = _mm256_set1_ps(gradients.dwy_dx);
        let dwz_vec = _mm256_set1_ps(gradients.dwz_dx);

        let offset_seq = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

        // Initial values as vectors
        let mut z_vec = _mm256_fmadd_ps(offset_seq, dz_vec, _mm256_set1_ps(z));
        let mut nx_vec = _mm256_fmadd_ps(offset_seq, dnx_vec, _mm256_set1_ps(nx));
        let mut ny_vec = _mm256_fmadd_ps(offset_seq, dny_vec, _mm256_set1_ps(ny));
        let mut nz_vec = _mm256_fmadd_ps(offset_seq, dnz_vec, _mm256_set1_ps(nz));
        let mut wx_vec = _mm256_fmadd_ps(offset_seq, dwx_vec, _mm256_set1_ps(wx));
        let mut wy_vec = _mm256_fmadd_ps(offset_seq, dwy_vec, _mm256_set1_ps(wy));
        let mut wz_vec = _mm256_fmadd_ps(offset_seq, dwz_vec, _mm256_set1_ps(wz));

        // Strides
        let dz_stride = _mm256_mul_ps(dz_vec, _mm256_set1_ps(8.0));
        let dnx_stride = _mm256_mul_ps(dnx_vec, _mm256_set1_ps(8.0));
        let dny_stride = _mm256_mul_ps(dny_vec, _mm256_set1_ps(8.0));
        let dnz_stride = _mm256_mul_ps(dnz_vec, _mm256_set1_ps(8.0));
        let dwx_stride = _mm256_mul_ps(dwx_vec, _mm256_set1_ps(8.0));
        let dwy_stride = _mm256_mul_ps(dwy_vec, _mm256_set1_ps(8.0));
        let dwz_stride = _mm256_mul_ps(dwz_vec, _mm256_set1_ps(8.0));

        // Constants
        let view_pos_x = _mm256_set1_ps(constants.view_pos.x);
        let view_pos_y = _mm256_set1_ps(constants.view_pos.y);
        let view_pos_z = _mm256_set1_ps(constants.view_pos.z);
        let neg_light_x = _mm256_set1_ps(constants.neg_light_dir.x);
        let neg_light_y = _mm256_set1_ps(constants.neg_light_dir.y);
        let neg_light_z = _mm256_set1_ps(constants.neg_light_dir.z);
        let light_color_x = _mm256_set1_ps(constants.light_color.x);
        let light_color_y = _mm256_set1_ps(constants.light_color.y);
        let light_color_z = _mm256_set1_ps(constants.light_color.z);
        let albedo_x = _mm256_set1_ps(constants.albedo.x);
        let albedo_y = _mm256_set1_ps(constants.albedo.y);
        let albedo_z = _mm256_set1_ps(constants.albedo.z);
        let ao_vec = _mm256_set1_ps(constants.ao);
        let metallic_vec = _mm256_set1_ps(constants.metallic);
        let f0_x = _mm256_set1_ps(constants.f0.x);
        let f0_y = _mm256_set1_ps(constants.f0.y);
        let f0_z = _mm256_set1_ps(constants.f0.z);
        let a2_vec = _mm256_set1_ps(constants.a2);
        let a2_minus_1_vec = _mm256_set1_ps(constants.a2_minus_1);
        let k_vec = _mm256_set1_ps(constants.k);
        let one_minus_k_vec = _mm256_set1_ps(constants.one_minus_k);
        let one = _mm256_set1_ps(1.0);
        let zero = _mm256_set1_ps(0.0);
        let pi_vec = _mm256_set1_ps(std::f32::consts::PI);
        let epsilon = _mm256_set1_ps(0.0001);
        let ambient_factor = _mm256_set1_ps(0.03);
        let scale_255 = _mm256_set1_ps(255.0);

        // Main loop
        let mut pixels_ptr = fb.as_mut_slice().as_mut_ptr().add(i);
        let mut depths_ptr = zb.as_mut_slice().as_mut_ptr().add(i);
        let mut remaining = count;

        while remaining >= 8 {
            let current_depths = _mm256_loadu_ps(depths_ptr);
            let mask = _mm256_cmp_ps(z_vec, current_depths, _CMP_LT_OQ);
            let mask_bits = _mm256_movemask_ps(mask);

            if mask_bits != 0 {
                // Normalize N
                // rsqrt(x) approx 1/sqrt(x). x * rsqrt(x) = sqrt(x).
                // normalize: v * rsqrt(dot(v, v))
                let n_dot_n = _mm256_fmadd_ps(
                    nx_vec,
                    nx_vec,
                    _mm256_fmadd_ps(ny_vec, ny_vec, _mm256_mul_ps(nz_vec, nz_vec)),
                );
                let inv_len_n = _mm256_rsqrt_ps(n_dot_n);
                let nx = _mm256_mul_ps(nx_vec, inv_len_n);
                let ny = _mm256_mul_ps(ny_vec, inv_len_n);
                let nz = _mm256_mul_ps(nz_vec, inv_len_n);

                // V = normalize(ViewPos - WorldPos)
                let vx_raw = _mm256_sub_ps(view_pos_x, wx_vec);
                let vy_raw = _mm256_sub_ps(view_pos_y, wy_vec);
                let vz_raw = _mm256_sub_ps(view_pos_z, wz_vec);
                let v_dot_v = _mm256_fmadd_ps(
                    vx_raw,
                    vx_raw,
                    _mm256_fmadd_ps(vy_raw, vy_raw, _mm256_mul_ps(vz_raw, vz_raw)),
                );
                let inv_len_v = _mm256_rsqrt_ps(v_dot_v);
                let vx = _mm256_mul_ps(vx_raw, inv_len_v);
                let vy = _mm256_mul_ps(vy_raw, inv_len_v);
                let vz = _mm256_mul_ps(vz_raw, inv_len_v);

                // L is constant (neg_light_x, y, z) - already normalized in constant struct assumption
                let lx = neg_light_x;
                let ly = neg_light_y;
                let lz = neg_light_z;

                // H = normalize(V + L)
                let hx_raw = _mm256_add_ps(vx, lx);
                let hy_raw = _mm256_add_ps(vy, ly);
                let hz_raw = _mm256_add_ps(vz, lz);
                let h_dot_h = _mm256_fmadd_ps(
                    hx_raw,
                    hx_raw,
                    _mm256_fmadd_ps(hy_raw, hy_raw, _mm256_mul_ps(hz_raw, hz_raw)),
                );
                let inv_len_h = _mm256_rsqrt_ps(h_dot_h);
                let hx = _mm256_mul_ps(hx_raw, inv_len_h);
                let hy = _mm256_mul_ps(hy_raw, inv_len_h);
                let hz = _mm256_mul_ps(hz_raw, inv_len_h);

                // Dot products
                // n_dot_h = max(0, dot(n, h))
                let n_dot_h_val =
                    _mm256_fmadd_ps(nx, hx, _mm256_fmadd_ps(ny, hy, _mm256_mul_ps(nz, hz)));
                let n_dot_h = _mm256_max_ps(zero, n_dot_h_val);

                // n_dot_v
                let n_dot_v_val =
                    _mm256_fmadd_ps(nx, vx, _mm256_fmadd_ps(ny, vy, _mm256_mul_ps(nz, vz)));
                let n_dot_v = _mm256_max_ps(zero, n_dot_v_val);

                // n_dot_l
                let n_dot_l_val =
                    _mm256_fmadd_ps(nx, lx, _mm256_fmadd_ps(ny, ly, _mm256_mul_ps(nz, lz)));
                let n_dot_l = _mm256_max_ps(zero, n_dot_l_val);

                // h_dot_v
                let h_dot_v_val =
                    _mm256_fmadd_ps(hx, vx, _mm256_fmadd_ps(hy, vy, _mm256_mul_ps(hz, vz)));
                let h_dot_v = _mm256_max_ps(zero, h_dot_v_val);

                // NDF (Distribution GGX)
                // denom = (n_dot_h^2 * (a2 - 1) + 1)
                // D = a2 / (PI * denom^2)
                let n_dot_h2 = _mm256_mul_ps(n_dot_h, n_dot_h);
                let ndf_denom_term = _mm256_fmadd_ps(n_dot_h2, a2_minus_1_vec, one);
                let ndf_denom =
                    _mm256_mul_ps(pi_vec, _mm256_mul_ps(ndf_denom_term, ndf_denom_term));
                let ndf_denom = _mm256_max_ps(ndf_denom, _mm256_set1_ps(0.000_000_1));
                let ndf = _mm256_div_ps(a2_vec, ndf_denom);

                // Geometry Smith
                // G = ggx1 * ggx2
                // ggx(dot) = dot / (dot * (1-k) + k)
                let ggx_denom_v = _mm256_fmadd_ps(n_dot_v, one_minus_k_vec, k_vec);
                let ggx_denom_l = _mm256_fmadd_ps(n_dot_l, one_minus_k_vec, k_vec);
                let ggx2 = _mm256_div_ps(
                    n_dot_v,
                    _mm256_max_ps(ggx_denom_v, _mm256_set1_ps(0.000_000_1)),
                );
                let ggx1 = _mm256_div_ps(
                    n_dot_l,
                    _mm256_max_ps(ggx_denom_l, _mm256_set1_ps(0.000_000_1)),
                );
                let g = _mm256_mul_ps(ggx1, ggx2);

                // Fresnel Schlick
                // F = F0 + (1 - F0) * (1 - h_dot_v)^5
                let one_minus_h_dot_v = _mm256_sub_ps(one, h_dot_v);
                let pow2 = _mm256_mul_ps(one_minus_h_dot_v, one_minus_h_dot_v);
                let pow4 = _mm256_mul_ps(pow2, pow2);
                let pow5 = _mm256_mul_ps(pow4, one_minus_h_dot_v);

                // lerp(a, b, t) = a + (b - a) * t
                // F = f0 + (1 - f0) * pow5
                let one_minus_f0_x = _mm256_sub_ps(one, f0_x);
                let one_minus_f0_y = _mm256_sub_ps(one, f0_y);
                let one_minus_f0_z = _mm256_sub_ps(one, f0_z);

                let f_x = _mm256_fmadd_ps(one_minus_f0_x, pow5, f0_x);
                let f_y = _mm256_fmadd_ps(one_minus_f0_y, pow5, f0_y);
                let f_z = _mm256_fmadd_ps(one_minus_f0_z, pow5, f0_z);

                // Specular
                // num = NDF * G * F
                // denom = 4 * n_dot_v * n_dot_l + 0.0001
                let ndf_g = _mm256_mul_ps(ndf, g);
                let spec_num_x = _mm256_mul_ps(ndf_g, f_x);
                let spec_num_y = _mm256_mul_ps(ndf_g, f_y);
                let spec_num_z = _mm256_mul_ps(ndf_g, f_z);

                let spec_denom = _mm256_fmadd_ps(
                    _mm256_set1_ps(4.0),
                    _mm256_mul_ps(n_dot_v, n_dot_l),
                    epsilon,
                );
                let inv_spec_denom = _mm256_div_ps(one, spec_denom);

                let specular_x = _mm256_mul_ps(spec_num_x, inv_spec_denom);
                let specular_y = _mm256_mul_ps(spec_num_y, inv_spec_denom);
                let specular_z = _mm256_mul_ps(spec_num_z, inv_spec_denom);

                // Diffuse
                // kS = F
                // kD = (1 - kS) * (1 - metallic)
                let one_minus_metallic = _mm256_sub_ps(one, metallic_vec);
                let kd_x = _mm256_mul_ps(_mm256_sub_ps(one, f_x), one_minus_metallic);
                let kd_y = _mm256_mul_ps(_mm256_sub_ps(one, f_y), one_minus_metallic);
                let kd_z = _mm256_mul_ps(_mm256_sub_ps(one, f_z), one_minus_metallic);

                let inv_pi = _mm256_div_ps(one, pi_vec);
                let diffuse_x = _mm256_mul_ps(kd_x, _mm256_mul_ps(albedo_x, inv_pi));
                let diffuse_y = _mm256_mul_ps(kd_y, _mm256_mul_ps(albedo_y, inv_pi));
                let diffuse_z = _mm256_mul_ps(kd_z, _mm256_mul_ps(albedo_z, inv_pi));

                // Lo = (diffuse + specular) * radiance * n_dot_l
                let radiance_n_dot_l_x = _mm256_mul_ps(light_color_x, n_dot_l);
                let radiance_n_dot_l_y = _mm256_mul_ps(light_color_y, n_dot_l);
                let radiance_n_dot_l_z = _mm256_mul_ps(light_color_z, n_dot_l);

                let lo_x = _mm256_mul_ps(_mm256_add_ps(diffuse_x, specular_x), radiance_n_dot_l_x);
                let lo_y = _mm256_mul_ps(_mm256_add_ps(diffuse_y, specular_y), radiance_n_dot_l_y);
                let lo_z = _mm256_mul_ps(_mm256_add_ps(diffuse_z, specular_z), radiance_n_dot_l_z);

                // Ambient
                let ambient_val = _mm256_mul_ps(ambient_factor, ao_vec);
                let ambient_x = _mm256_mul_ps(ambient_val, albedo_x);
                let ambient_y = _mm256_mul_ps(ambient_val, albedo_y);
                let ambient_z = _mm256_mul_ps(ambient_val, albedo_z);

                let c_x = _mm256_add_ps(ambient_x, lo_x);
                let c_y = _mm256_add_ps(ambient_y, lo_y);
                let c_z = _mm256_add_ps(ambient_z, lo_z);

                // Tone mapping (Reinhard)
                // c / (c + 1)
                let tm_x = _mm256_div_ps(c_x, _mm256_add_ps(c_x, one));
                let tm_y = _mm256_div_ps(c_y, _mm256_add_ps(c_y, one));
                let tm_z = _mm256_div_ps(c_z, _mm256_add_ps(c_z, one));

                // Gamma (sqrt)
                let final_x = _mm256_sqrt_ps(tm_x);
                let final_y = _mm256_sqrt_ps(tm_y);
                let final_z = _mm256_sqrt_ps(tm_z);

                // Convert to u32 0xAARRGGBB
                // r = final_x * 255
                let r_f = _mm256_mul_ps(final_x, scale_255);
                let g_f = _mm256_mul_ps(final_y, scale_255);
                let b_f = _mm256_mul_ps(final_z, scale_255);

                let r_i = _mm256_cvtps_epi32(r_f);
                let g_i = _mm256_cvtps_epi32(g_f);
                let b_i = _mm256_cvtps_epi32(b_f);

                // Pack
                // Alpha is FF
                let alpha = _mm256_set1_epi32(0xFF00_0000_u32 as i32);
                // (r << 16) | (g << 8) | b
                let r_shift = _mm256_slli_epi32(r_i, 16);
                let g_shift = _mm256_slli_epi32(g_i, 8);
                let rgb = _mm256_or_si256(_mm256_or_si256(r_shift, g_shift), b_i);
                let final_colors = _mm256_or_si256(rgb, alpha);

                // Masked store
                // We need to write depths and colors where z < depth
                _mm256_storeu_ps(depths_ptr, _mm256_blendv_ps(current_depths, z_vec, mask));

                #[allow(clippy::cast_ptr_alignment)]
                let old_pixels = _mm256_loadu_si256(pixels_ptr.cast::<__m256i>());
                let final_pixels_ps = _mm256_blendv_ps(
                    _mm256_castsi256_ps(old_pixels),
                    _mm256_castsi256_ps(final_colors),
                    mask,
                );
                #[allow(clippy::cast_ptr_alignment)]
                _mm256_storeu_si256(
                    pixels_ptr.cast::<__m256i>(),
                    _mm256_castps_si256(final_pixels_ps),
                );
            }

            // Advance
            z_vec = _mm256_add_ps(z_vec, dz_stride);
            nx_vec = _mm256_add_ps(nx_vec, dnx_stride);
            ny_vec = _mm256_add_ps(ny_vec, dny_stride);
            nz_vec = _mm256_add_ps(nz_vec, dnz_stride);
            wx_vec = _mm256_add_ps(wx_vec, dwx_stride);
            wy_vec = _mm256_add_ps(wy_vec, dwy_stride);
            wz_vec = _mm256_add_ps(wz_vec, dwz_stride);

            pixels_ptr = pixels_ptr.add(8);
            depths_ptr = depths_ptr.add(8);
            xs += 8;
            remaining -= 8;
        }

        // Scalar fallback for remainder
        if remaining > 0 {
            // Recalculate scalar start values at current xs
            // Note: xs has been incremented
            // But we can just use the gradients from the original start to calculate current values
            // or update scalar state.
            // Let's update scalar state.
            // Actually, z_vec holds the next 8 values.
            // But we can just restart from original scalar params + offset.
            let diff = (xs - x_start) as f32;
            let z_s = start.z + diff * gradients.dz_dx;
            let nx_s = start.nx + diff * gradients.dnx_dx;
            let ny_s = start.ny + diff * gradients.dny_dx;
            let nz_s = start.nz + diff * gradients.dnz_dx;
            let wx_s = start.wx + diff * gradients.dwx_dx;
            let wy_s = start.wy + diff * gradients.dwy_dx;
            let wz_s = start.wz + diff * gradients.dwz_dx;

            draw_scanline_pbr_scalar(
                fb,
                zb,
                y,
                xs,
                xe,
                PbrSpanStart {
                    z: z_s,
                    nx: nx_s,
                    ny: ny_s,
                    nz: nz_s,
                    wx: wx_s,
                    wy: wy_s,
                    wz: wz_s,
                },
                gradients,
                constants,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_scanline_pbr_scalar(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PbrSpanStart,
    gradients: &PbrGradients,
    constants: &PbrConstants,
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut wx = start.wx;
    let mut wy = start.wy;
    let mut wz = start.wz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        wx += diff_f * gradients.dwx_dx;
        wy += diff_f * gradients.dwy_dx;
        wz += diff_f * gradients.dwz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY: Clamped above.
    let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
    let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

    let l = constants.neg_light_dir;

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Reconstruct vectors
            // Normal needs normalization after interpolation
            let n = Vec3::new(nx, ny, nz).fast_normalize();
            let world_pos = Vec3::new(wx, wy, wz);

            let v = (constants.view_pos - world_pos).fast_normalize();
            // l is constant
            let h = (v + l).fast_normalize();

            // Cook-Torrance BRDF
            let n_dot_h = n.dot(h).max(0.0);
            let n_dot_h2 = n_dot_h * n_dot_h;
            let ndf = distribution_ggx_optimized(n_dot_h2, constants);

            let n_dot_v = n.dot(v).max(0.0);
            let n_dot_l = n.dot(l).max(0.0);
            let g = geometry_smith_optimized(n_dot_v, n_dot_l, constants);

            let h_dot_v = h.dot(v).max(0.0);
            let f = fresnel_schlick(h_dot_v, constants.f0);

            let numerator = f * ndf * g;
            let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001;
            let specular = numerator * (1.0 / denominator);

            let k_s = f;
            let k_d = (Vec3::ONE - k_s) * (1.0 - constants.metallic);

            // lo = (k_d * albedo / PI + specular) * radiance * n_dot_l
            let diffuse = k_d * constants.albedo * (1.0 / PI);
            let lo = (diffuse + specular) * constants.light_color * n_dot_l;

            let ambient = Vec3::new(0.03, 0.03, 0.03) * constants.albedo * constants.ao;
            let color = ambient + lo;

            // Tone mapping (Reinhard)
            // mapped = color / (color + 1.0)
            // Avoid creating intermediate Vec3s
            let cx = color.x;
            let cy = color.y;
            let cz = color.z;
            let mx = cx / (cx + 1.0);
            let my = cy / (cy + 1.0);
            let mz = cz / (cz + 1.0);

            // Gamma correction (Approximation Gamma 2.0 using sqrt)
            // fast_inv_sqrt is for 1/sqrt. sqrt is fast.
            let corrected = Vec3::new(mx.sqrt(), my.sqrt(), mz.sqrt());

            *pixel = color_to_u32_scaled(corrected * 255.0);
        }

        z += gradients.dz_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        wx += gradients.dwx_dx;
        wy += gradients.dwy_dx;
        wz += gradients.dwz_dx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Vec3;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn test_pbr_simd_matches_scalar() {
        let width = 32;
        let height = 1; // Single scanline
        let mut fb_scalar = Framebuffer::new(width, height).unwrap();
        let mut zb_scalar = ZBuffer::new(width, height).unwrap();
        let mut fb_simd = Framebuffer::new(width, height).unwrap();
        let mut zb_simd = ZBuffer::new(width, height).unwrap();

        // Initialize Z-buffers
        zb_scalar.clear();
        zb_simd.clear();
        fb_scalar.clear(0);
        fb_simd.clear(0);

        let start = PbrSpanStart {
            z: 5.0,
            nx: 0.0,
            ny: 0.0,
            nz: 1.0,
            wx: 0.0,
            wy: 0.0,
            wz: 0.0,
        };

        let gradients = PbrGradients {
            dz_dx: 0.1,
            dnx_dx: 0.01,
            dny_dx: 0.0,
            dnz_dx: 0.0,
            dwx_dx: 0.1,
            dwy_dx: 0.0,
            dwz_dx: 0.0,
        };

        let constants = PbrConstants {
            a2: 0.04,
            a2_minus_1: -0.96,
            k: 0.125,
            one_minus_k: 0.875,
            f0: Vec3::new(0.04, 0.04, 0.04),
            dielectric_f0: Vec3::new(0.04, 0.04, 0.04),
            neg_light_dir: Vec3::new(0.0, 0.0, 1.0),
            light_color: Vec3::new(1.0, 1.0, 1.0),
            view_pos: Vec3::new(0.0, 0.0, 10.0),
            albedo: Vec3::new(1.0, 0.0, 0.0),
            metallic: 0.0,
            ao: 1.0,
        };

        let y = 0;
        let x_start = 0;
        let x_end = 31; // Full width

        // Run Scalar
        draw_scanline_pbr_scalar(
            &mut fb_scalar,
            &mut zb_scalar,
            y,
            x_start,
            x_end,
            start,
            &gradients,
            &constants,
        );

        // Run SIMD (if available)
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        if (x_end - x_start + 1) >= 32
            && is_x86_feature_detected!("avx2")
            && is_x86_feature_detected!("fma")
        {
            unsafe {
                draw_scanline_pbr_simd(
                    &mut fb_simd,
                    &mut zb_simd,
                    y,
                    x_start,
                    x_end,
                    start,
                    &gradients,
                    &constants,
                );
            }
        } else {
            // If SIMD not available, run scalar again to mimic fallback behavior
            draw_scanline_pbr_scalar(
                &mut fb_simd,
                &mut zb_simd,
                y,
                x_start,
                x_end,
                start,
                &gradients,
                &constants,
            );
        }

        // If SIMD feature is NOT enabled/compiled, we can't test draw_scanline_pbr_simd
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            draw_scanline_pbr_scalar(
                &mut fb_simd,
                &mut zb_simd,
                y,
                x_start,
                x_end,
                start,
                &gradients,
                &constants,
            );
        }

        // Compare results
        let scalar_pixels = fb_scalar.as_slice();
        let simd_pixels = fb_simd.as_slice();
        let scalar_depths = zb_scalar.as_slice();
        let simd_depths = zb_simd.as_slice();

        for i in 0..width as usize {
            // Decompose colors
            let s = scalar_pixels[i];
            let v = simd_pixels[i];
            let sa = (s >> 24) & 0xFF;
            let sr = (s >> 16) & 0xFF;
            let sg = (s >> 8) & 0xFF;
            let sb = s & 0xFF;

            let va = (v >> 24) & 0xFF;
            let vr = (v >> 16) & 0xFF;
            let vg = (v >> 8) & 0xFF;
            let vb = v & 0xFF;

            // Allow +/- 1 difference per channel due to rsqrt vs sqrt precision
            let tol = 1;
            assert!(
                sa == va
                    && (sr as i32 - vr as i32).abs() <= tol
                    && (sg as i32 - vg as i32).abs() <= tol
                    && (sb as i32 - vb as i32).abs() <= tol,
                "Pixel mismatch at index {i}: scalar={s:08x} (a{sa}, r{sr}, g{sg}, b{sb}), simd={v:08x} (a{va}, r{vr}, g{vg}, b{vb})"
            );

            assert!(
                (scalar_depths[i] - simd_depths[i]).abs() < 1e-4,
                "Depth mismatch at index {}: scalar={}, simd={}",
                i,
                scalar_depths[i],
                simd_depths[i]
            );
        }
    }
}
