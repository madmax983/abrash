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
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Vec3;
//! use abrash::rasterizer::pbr::fill_triangle_pbr;
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

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

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

#[derive(Clone, Copy)]
struct PbrSpanStart {
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

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
    constants.a2 / denom.max(0.0000001)
}

// Geometry Schlick-GGX
fn geometry_schlick_ggx_optimized(n_dot_v: f32, constants: &PbrConstants) -> f32 {
    let nom = n_dot_v;
    let denom = n_dot_v * constants.one_minus_k + constants.k;
    nom / denom.max(0.0000001)
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

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
                draw_scanline_pbr_simd(
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
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    let l = constants.neg_light_dir;

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Reconstruct vectors
            // Normal needs normalization after interpolation
            let n = Vec3::new(nx, ny, nz).fast_normalize();
            let world_pos = Vec3::new(wx, wy, wz);

            let v = (constants.view_pos - world_pos).normalize();
            // l is constant
            let h = (v + l).normalize();

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
            let denom = color + Vec3::ONE;
            let mapped = Vec3::new(color.x / denom.x, color.y / denom.y, color.z / denom.z);
            // Gamma correction (Approximation Gamma 2.0 using sqrt)
            // fast_inv_sqrt is for 1/sqrt. sqrt is fast.
            let corrected = Vec3::new(mapped.x.sqrt(), mapped.y.sqrt(), mapped.z.sqrt());

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

// AVX2 Implementation
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(clippy::too_many_arguments, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn draw_scanline_pbr_avx2(
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

    // Broadcast gradients
    let v_dz_dx = _mm256_set1_ps(gradients.dz_dx);
    let v_dnx_dx = _mm256_set1_ps(gradients.dnx_dx);
    let v_dny_dx = _mm256_set1_ps(gradients.dny_dx);
    let v_dnz_dx = _mm256_set1_ps(gradients.dnz_dx);
    let v_dwx_dx = _mm256_set1_ps(gradients.dwx_dx);
    let v_dwy_dx = _mm256_set1_ps(gradients.dwy_dx);
    let v_dwz_dx = _mm256_set1_ps(gradients.dwz_dx);

    // Broadcast constants
    let v_view_pos_x = _mm256_set1_ps(constants.view_pos.x);
    let v_view_pos_y = _mm256_set1_ps(constants.view_pos.y);
    let v_view_pos_z = _mm256_set1_ps(constants.view_pos.z);

    let v_light_dir_neg_x = _mm256_set1_ps(constants.neg_light_dir.x);
    let v_light_dir_neg_y = _mm256_set1_ps(constants.neg_light_dir.y);
    let v_light_dir_neg_z = _mm256_set1_ps(constants.neg_light_dir.z);

    let v_light_color_x = _mm256_set1_ps(constants.light_color.x);
    let v_light_color_y = _mm256_set1_ps(constants.light_color.y);
    let v_light_color_z = _mm256_set1_ps(constants.light_color.z);

    let v_albedo_x = _mm256_set1_ps(constants.albedo.x);
    let v_albedo_y = _mm256_set1_ps(constants.albedo.y);
    let v_albedo_z = _mm256_set1_ps(constants.albedo.z);

    // v_roughness unused, using precomputed
    let v_a2 = _mm256_set1_ps(constants.a2);
    let v_a2_minus_1 = _mm256_set1_ps(constants.a2_minus_1);
    let v_k = _mm256_set1_ps(constants.k);
    let v_one_minus_k = _mm256_set1_ps(constants.one_minus_k);
    let v_f0_x = _mm256_set1_ps(constants.f0.x);
    let v_f0_y = _mm256_set1_ps(constants.f0.y);
    let v_f0_z = _mm256_set1_ps(constants.f0.z);
    let v_metallic = _mm256_set1_ps(constants.metallic);
    let v_ao = _mm256_set1_ps(constants.ao);
    let v_pi = _mm256_set1_ps(PI);
    let v_one = _mm256_set1_ps(1.0);
    let v_zero = _mm256_setzero_ps();
    let v_epsilon = _mm256_set1_ps(0.0001);
    let v_epsilon_denom = _mm256_set1_ps(0.0000001);
    let v_ambient_scale = _mm256_set1_ps(0.03);
    let v_255 = _mm256_set1_ps(255.0);

    // Initial offsets for SIMD (0, 1, 2, ..., 7)
    let v_idx_offset = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

    // Adjust starting values to be vector of 8
    let mut v_z = _mm256_fmadd_ps(v_dz_dx, v_idx_offset, _mm256_set1_ps(z));
    let mut v_nx = _mm256_fmadd_ps(v_dnx_dx, v_idx_offset, _mm256_set1_ps(nx));
    let mut v_ny = _mm256_fmadd_ps(v_dny_dx, v_idx_offset, _mm256_set1_ps(ny));
    let mut v_nz = _mm256_fmadd_ps(v_dnz_dx, v_idx_offset, _mm256_set1_ps(nz));
    let mut v_wx = _mm256_fmadd_ps(v_dwx_dx, v_idx_offset, _mm256_set1_ps(wx));
    let mut v_wy = _mm256_fmadd_ps(v_dwy_dx, v_idx_offset, _mm256_set1_ps(wy));
    let mut v_wz = _mm256_fmadd_ps(v_dwz_dx, v_idx_offset, _mm256_set1_ps(wz));

    // Gradients * 8 for loop increment
    let v_dz_dx_8 = _mm256_mul_ps(v_dz_dx, _mm256_set1_ps(8.0));
    let v_dnx_dx_8 = _mm256_mul_ps(v_dnx_dx, _mm256_set1_ps(8.0));
    let v_dny_dx_8 = _mm256_mul_ps(v_dny_dx, _mm256_set1_ps(8.0));
    let v_dnz_dx_8 = _mm256_mul_ps(v_dnz_dx, _mm256_set1_ps(8.0));
    let v_dwx_dx_8 = _mm256_mul_ps(v_dwx_dx, _mm256_set1_ps(8.0));
    let v_dwy_dx_8 = _mm256_mul_ps(v_dwy_dx, _mm256_set1_ps(8.0));
    let v_dwz_dx_8 = _mm256_mul_ps(v_dwz_dx, _mm256_set1_ps(8.0));

    let mut x = xs;

    // Loop 8 by 8
    while x <= xe - 7 {
        let idx = y_offset + x as usize;
        let zb_ptr = zb.as_mut_slice().as_mut_ptr().add(idx);
        let fb_ptr = fb.as_mut_slice().as_mut_ptr().add(idx);

        // Load Z buffer
        let v_depth_val = _mm256_loadu_ps(zb_ptr);

        // Z test: z < depth_val
        let mask = _mm256_cmp_ps(v_z, v_depth_val, _CMP_LT_OQ);
        let mask_int = _mm256_movemask_ps(mask);

        if mask_int != 0 {
            // Update Z buffer where mask is true
            let v_new_depth = _mm256_blendv_ps(v_depth_val, v_z, mask);
            _mm256_storeu_ps(zb_ptr, v_new_depth);

            // --- PBR Calculation ---

            // Normalize Normal (N)
            let n_dot_n = _mm256_fmadd_ps(
                v_nx, v_nx,
                _mm256_fmadd_ps(v_ny, v_ny, _mm256_mul_ps(v_nz, v_nz)),
            );
            let inv_len_n = _mm256_rsqrt_ps(n_dot_n);
            let v_n_x = _mm256_mul_ps(v_nx, inv_len_n);
            let v_n_y = _mm256_mul_ps(v_ny, inv_len_n);
            let v_n_z = _mm256_mul_ps(v_nz, inv_len_n);

            // View Vector (V)
            let v_v_x_raw = _mm256_sub_ps(v_view_pos_x, v_wx);
            let v_v_y_raw = _mm256_sub_ps(v_view_pos_y, v_wy);
            let v_v_z_raw = _mm256_sub_ps(v_view_pos_z, v_wz);
            let v_dot_v = _mm256_fmadd_ps(
                v_v_x_raw, v_v_x_raw,
                _mm256_fmadd_ps(v_v_y_raw, v_v_y_raw, _mm256_mul_ps(v_v_z_raw, v_v_z_raw)),
            );
            let inv_len_v = _mm256_rsqrt_ps(v_dot_v);
            let v_v_x = _mm256_mul_ps(v_v_x_raw, inv_len_v);
            let v_v_y = _mm256_mul_ps(v_v_y_raw, inv_len_v);
            let v_v_z = _mm256_mul_ps(v_v_z_raw, inv_len_v);

            // Half Vector (H)
            let v_h_x_raw = _mm256_add_ps(v_v_x, v_light_dir_neg_x);
            let v_h_y_raw = _mm256_add_ps(v_v_y, v_light_dir_neg_y);
            let v_h_z_raw = _mm256_add_ps(v_v_z, v_light_dir_neg_z);
            let h_dot_h = _mm256_fmadd_ps(
                v_h_x_raw, v_h_x_raw,
                _mm256_fmadd_ps(v_h_y_raw, v_h_y_raw, _mm256_mul_ps(v_h_z_raw, v_h_z_raw)),
            );
            let inv_len_h = _mm256_rsqrt_ps(h_dot_h);
            let v_h_x = _mm256_mul_ps(v_h_x_raw, inv_len_h);
            let v_h_y = _mm256_mul_ps(v_h_y_raw, inv_len_h);
            let v_h_z = _mm256_mul_ps(v_h_z_raw, inv_len_h);

            // N dot H
            let n_dot_h = _mm256_fmadd_ps(
                v_n_x, v_h_x,
                _mm256_fmadd_ps(v_n_y, v_h_y, _mm256_mul_ps(v_n_z, v_h_z)),
            );
            let n_dot_h = _mm256_max_ps(n_dot_h, v_zero);
            let n_dot_h2 = _mm256_mul_ps(n_dot_h, n_dot_h);

            // NDF
            let denom = _mm256_fmadd_ps(n_dot_h2, v_a2_minus_1, v_one);
            let denom_sq = _mm256_mul_ps(denom, denom);
            let denom_final = _mm256_mul_ps(v_pi, denom_sq);
            let denom_final = _mm256_max_ps(denom_final, v_epsilon_denom);
            let ndf = _mm256_div_ps(v_a2, denom_final);

            // N dot V, N dot L
            let n_dot_v = _mm256_fmadd_ps(
                v_n_x, v_v_x,
                _mm256_fmadd_ps(v_n_y, v_v_y, _mm256_mul_ps(v_n_z, v_v_z)),
            );
            let n_dot_v = _mm256_max_ps(n_dot_v, v_zero);

            let n_dot_l = _mm256_fmadd_ps(
                v_n_x, v_light_dir_neg_x,
                _mm256_fmadd_ps(v_n_y, v_light_dir_neg_y, _mm256_mul_ps(v_n_z, v_light_dir_neg_z)),
            );
            let n_dot_l = _mm256_max_ps(n_dot_l, v_zero);

            // Geometry Smith
            let denom_v = _mm256_fmadd_ps(n_dot_v, v_one_minus_k, v_k);
            let ggx1 = _mm256_div_ps(n_dot_v, _mm256_max_ps(denom_v, v_epsilon_denom));

            let denom_l = _mm256_fmadd_ps(n_dot_l, v_one_minus_k, v_k);
            let ggx2 = _mm256_div_ps(n_dot_l, _mm256_max_ps(denom_l, v_epsilon_denom));
            let g = _mm256_mul_ps(ggx1, ggx2);

            // Fresnel Schlick
            let h_dot_v = _mm256_fmadd_ps(
                v_h_x, v_v_x,
                _mm256_fmadd_ps(v_h_y, v_v_y, _mm256_mul_ps(v_h_z, v_v_z)),
            );
            let h_dot_v = _mm256_max_ps(h_dot_v, v_zero);

            let one_minus_cos = _mm256_sub_ps(v_one, h_dot_v);
            let pow2 = _mm256_mul_ps(one_minus_cos, one_minus_cos);
            let pow4 = _mm256_mul_ps(pow2, pow2);
            let pow5 = _mm256_mul_ps(pow4, one_minus_cos);

            let v_f_x = _mm256_fmadd_ps(_mm256_sub_ps(v_one, v_f0_x), pow5, v_f0_x);
            let v_f_y = _mm256_fmadd_ps(_mm256_sub_ps(v_one, v_f0_y), pow5, v_f0_y);
            let v_f_z = _mm256_fmadd_ps(_mm256_sub_ps(v_one, v_f0_z), pow5, v_f0_z);

            // Specular
            let ndf_g = _mm256_mul_ps(ndf, g);
            let numerator_x = _mm256_mul_ps(v_f_x, ndf_g);
            let numerator_y = _mm256_mul_ps(v_f_y, ndf_g);
            let numerator_z = _mm256_mul_ps(v_f_z, ndf_g);

            let denominator = _mm256_mul_ps(_mm256_set1_ps(4.0), _mm256_mul_ps(n_dot_v, n_dot_l));
            let denominator = _mm256_add_ps(denominator, v_epsilon);
            let inv_denominator = _mm256_rcp_ps(denominator);

            let specular_x = _mm256_mul_ps(numerator_x, inv_denominator);
            let specular_y = _mm256_mul_ps(numerator_y, inv_denominator);
            let specular_z = _mm256_mul_ps(numerator_z, inv_denominator);

            // Diffuse
            let ks_x = v_f_x;
            let ks_y = v_f_y;
            let ks_z = v_f_z;

            let one_minus_metallic = _mm256_sub_ps(v_one, v_metallic);
            let kd_x = _mm256_mul_ps(_mm256_sub_ps(v_one, ks_x), one_minus_metallic);
            let kd_y = _mm256_mul_ps(_mm256_sub_ps(v_one, ks_y), one_minus_metallic);
            let kd_z = _mm256_mul_ps(_mm256_sub_ps(v_one, ks_z), one_minus_metallic);

            let inv_pi = _mm256_set1_ps(1.0 / PI);
            let diffuse_x = _mm256_mul_ps(kd_x, _mm256_mul_ps(v_albedo_x, inv_pi));
            let diffuse_y = _mm256_mul_ps(kd_y, _mm256_mul_ps(v_albedo_y, inv_pi));
            let diffuse_z = _mm256_mul_ps(kd_z, _mm256_mul_ps(v_albedo_z, inv_pi));

            // Final Color
            let lo_x = _mm256_mul_ps(
                _mm256_add_ps(diffuse_x, specular_x),
                _mm256_mul_ps(v_light_color_x, n_dot_l),
            );
            let lo_y = _mm256_mul_ps(
                _mm256_add_ps(diffuse_y, specular_y),
                _mm256_mul_ps(v_light_color_y, n_dot_l),
            );
            let lo_z = _mm256_mul_ps(
                _mm256_add_ps(diffuse_z, specular_z),
                _mm256_mul_ps(v_light_color_z, n_dot_l),
            );

            let ambient_factor = _mm256_mul_ps(v_ambient_scale, v_ao);
            let ambient_x = _mm256_mul_ps(v_albedo_x, ambient_factor);
            let ambient_y = _mm256_mul_ps(v_albedo_y, ambient_factor);
            let ambient_z = _mm256_mul_ps(v_albedo_z, ambient_factor);

            let color_x = _mm256_add_ps(ambient_x, lo_x);
            let color_y = _mm256_add_ps(ambient_y, lo_y);
            let color_z = _mm256_add_ps(ambient_z, lo_z);

            // Tone Mapping
            let denom_tm_x = _mm256_add_ps(color_x, v_one);
            let denom_tm_y = _mm256_add_ps(color_y, v_one);
            let denom_tm_z = _mm256_add_ps(color_z, v_one);

            let mapped_x = _mm256_div_ps(color_x, denom_tm_x);
            let mapped_y = _mm256_div_ps(color_y, denom_tm_y);
            let mapped_z = _mm256_div_ps(color_z, denom_tm_z);

            // Gamma Correction
            let corrected_x = _mm256_sqrt_ps(mapped_x);
            let corrected_y = _mm256_sqrt_ps(mapped_y);
            let corrected_z = _mm256_sqrt_ps(mapped_z);

            // Packing
            let r = _mm256_mul_ps(corrected_x, v_255);
            let g = _mm256_mul_ps(corrected_y, v_255);
            let b = _mm256_mul_ps(corrected_z, v_255);

            let r_i = _mm256_cvttps_epi32(r);
            let g_i = _mm256_cvttps_epi32(g);
            let b_i = _mm256_cvttps_epi32(b);

            let alpha = _mm256_set1_epi32(255i32 << 24);
            let r_shifted = _mm256_slli_epi32(r_i, 16);
            let g_shifted = _mm256_slli_epi32(g_i, 8);

            let final_color = _mm256_or_si256(
                alpha,
                _mm256_or_si256(r_shifted, _mm256_or_si256(g_shifted, b_i)),
            );

            if mask_int == 0xFF {
                _mm256_storeu_si256(fb_ptr as *mut __m256i, final_color);
            } else {
                let mask_i = _mm256_castps_si256(mask);
                _mm256_maskstore_epi32(fb_ptr as *mut i32, mask_i, final_color);
            }
        }

        // Increment
        v_z = _mm256_add_ps(v_z, v_dz_dx_8);
        v_nx = _mm256_add_ps(v_nx, v_dnx_dx_8);
        v_ny = _mm256_add_ps(v_ny, v_dny_dx_8);
        v_nz = _mm256_add_ps(v_nz, v_dnz_dx_8);
        v_wx = _mm256_add_ps(v_wx, v_dwx_dx_8);
        v_wy = _mm256_add_ps(v_wy, v_dwy_dx_8);
        v_wz = _mm256_add_ps(v_wz, v_dwz_dx_8);

        x += 8;
    }

    // Epilogue setup
    let processed_pixels = x - xs;
    if processed_pixels > 0 {
        let diff_f = processed_pixels as f32;
        z += diff_f * gradients.dz_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        wx += diff_f * gradients.dwx_dx;
        wy += diff_f * gradients.dwy_dx;
        wz += diff_f * gradients.dwz_dx;
        xs = x;
    }

    // Call scalar for remaining
    draw_scanline_pbr(
        fb,
        zb,
        y,
        xs,
        x_end,
        PbrSpanStart {
            z,
            nx,
            ny,
            nz,
            wx,
            wy,
            wz,
        },
        gradients,
        constants,
    );
}

// Dispatcher
#[allow(clippy::too_many_arguments)]
fn draw_scanline_pbr_simd(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PbrSpanStart,
    gradients: &PbrGradients,
    constants: &PbrConstants,
) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            unsafe {
                draw_scanline_pbr_avx2(fb, zb, y, x_start, x_end, start, gradients, constants);
            }
        } else {
            draw_scanline_pbr(fb, zb, y, x_start, x_end, start, gradients, constants);
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        draw_scanline_pbr(fb, zb, y, x_start, x_end, start, gradients, constants);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::zbuffer::ZBuffer;
    use crate::math::Vec3;

    #[test]
    fn test_pbr_simd_matches_scalar() {
        let width = 64;
        let height = 64;
        let mut fb_scalar = Framebuffer::new(width, height).unwrap();
        let mut zb_scalar = ZBuffer::new(width, height).unwrap();
        let mut fb_simd = Framebuffer::new(width, height).unwrap();
        let mut zb_simd = ZBuffer::new(width, height).unwrap();

        let start = PbrSpanStart {
            z: 10.0,
            nx: 0.0, ny: 0.0, nz: 1.0,
            wx: 0.0, wy: 0.0, wz: 0.0,
        };

        let gradients = PbrGradients {
            dz_dx: 0.001,
            dnx_dx: 0.0, dny_dx: 0.0, dnz_dx: 0.0,
            dwx_dx: 0.1, dwy_dx: 0.0, dwz_dx: 0.0,
        };

        let constants = PbrConstants {
            a2: 0.5,
            a2_minus_1: -0.5,
            k: 0.5,
            one_minus_k: 0.5,
            f0: Vec3::new(0.04, 0.04, 0.04),
            dielectric_f0: Vec3::new(0.04, 0.04, 0.04),
            neg_light_dir: Vec3::new(0.0, 0.0, 1.0),
            light_color: Vec3::new(1.0, 1.0, 1.0),
            view_pos: Vec3::new(0.0, 0.0, 10.0),
            albedo: Vec3::new(1.0, 0.0, 0.0),
            metallic: 0.0,
            ao: 1.0,
        };

        let y = 32;
        let x_start = 10;
        let x_end = 50;

        // Scalar run
        draw_scanline_pbr(
            &mut fb_scalar,
            &mut zb_scalar,
            y,
            x_start,
            x_end,
            start,
            &gradients,
            &constants,
        );

        // SIMD run
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

        // Compare
        let scalar_slice = fb_scalar.as_slice();
        let simd_slice = fb_simd.as_slice();

        // Check if anything was drawn in scalar
        let drawn_scalar = scalar_slice.iter().any(|&p| p != 0);
        assert!(drawn_scalar, "Scalar implementation did not draw anything!");

        // Assuming SIMD panics, this part won't be reached initially.
        // Once implemented, it should verify match.
        fn color_diff(c1: u32, c2: u32) -> u32 {
            let r1 = (c1 >> 16) & 0xFF;
            let g1 = (c1 >> 8) & 0xFF;
            let b1 = c1 & 0xFF;

            let r2 = (c2 >> 16) & 0xFF;
            let g2 = (c2 >> 8) & 0xFF;
            let b2 = c2 & 0xFF;

            (r1 as i32 - r2 as i32).abs() as u32
                + (g1 as i32 - g2 as i32).abs() as u32
                + (b1 as i32 - b2 as i32).abs() as u32
        }

        for (i, (s, d)) in scalar_slice.iter().zip(simd_slice.iter()).enumerate() {
            if *s != *d {
                let diff = color_diff(*s, *d);
                // Allow some tolerance per channel due to rcp/rsqrt
                // Total diff <= 3 (e.g. 1 per channel)
                if diff > 3 {
                    panic!(
                        "Mismatch at index {}: Scalar {:08X} vs SIMD {:08X} (diff {})",
                        i, s, d, diff
                    );
                }
            }
        }
    }
}
