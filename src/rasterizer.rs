//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).
//!
//! # Rasterization Rules
//!
//! This module implements a standard **Scanline Rasterization** algorithm.
//!
//! 1.  **Triangle Setup**: Vertices are sorted by Y-coordinate.
//! 2.  **Edge Walking**: The left and right edges of the triangle are traced row by row.
//! 3.  **Span Filling**: For each scanline, a horizontal span of pixels is filled between the left and right edges.
//! 4.  **Top-Left Rule**: To prevent double-drawing on shared edges, the rasterizer follows standard fill conventions.
//!
//! # Performance
//!
//! *   **Fixed-Point Math**: Internal interpolation often uses 16.16 fixed-point arithmetic for speed.
//! *   **Z-Buffering**: Depth testing is performed per-pixel.
//! *   **Clipping**: Triangles are clipped to the view frustum before rasterization to ensure safety.

use crate::clipping::{clip_line_to_frustum, clip_triangle_to_frustum};
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec2, Vec3, Vec4, fast_inv_sqrt, project_to_screen_optimized};
use crate::texture::{FilterMode, Texture, blend_four_way, blend_swar};
use crate::zbuffer::ZBuffer;

/// Helper to ensure buffer dimensions match
#[inline]
fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
    assert_eq!(
        fb.width(),
        zb.width(),
        "Framebuffer and ZBuffer widths must match"
    );
    assert_eq!(
        fb.height(),
        zb.height(),
        "Framebuffer and ZBuffer heights must match"
    );
}

/// Helper to prepare scanline slices.
/// Returns (`fb_slice`, `zb_slice`, `adjusted_z_start`) or `None` if off-screen.
#[inline(always)]
fn prepare_scanline<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    mut z: f32,
    dz_dx: f32,
) -> Option<(&'a mut [u32], &'a mut [f32], f32)> {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    if xs < 0 {
        let diff = -xs as f32;
        z += diff * dz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return None;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY:
    // 1. xs and xe are clamped to [0, width-1].
    // 2. y is assumed to be within bounds by caller (clamped in fill_triangle).
    // 3. start_idx <= end_idx because xs <= xe.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    Some((fb_slice, zb_slice, z))
}

/// Helper to sort 3 vertices by Y coordinate
///
/// Optimization: Uses a manual sorting network to avoid the heap allocation
/// incurred by `slice::sort_by_key` for small arrays.
pub(crate) fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
where
    F: Fn(&T) -> i32,
{
    // Manual 3-step sort to avoid allocation
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
    if get_y(&verts[1]) > get_y(&verts[2]) {
        verts.swap(1, 2);
    }
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
}

/// Checks if a triangle is backfacing (or degenerate)
///
/// Uses the 2D cross product of the screen-space edges.
/// Returns true if the triangle should be culled (ccw winding for front faces).
#[inline(always)]
pub(crate) fn is_backface(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
    let ux = i64::from(p1.x) - i64::from(p0.x);
    let uy = i64::from(p1.y) - i64::from(p0.y);
    let vx = i64::from(p2.x) - i64::from(p0.x);
    let vy = i64::from(p2.y) - i64::from(p0.y);
    // Use i128 to prevent overflow during cross product calculation for large coordinates
    let nz = i128::from(ux) * i128::from(vy) - i128::from(uy) * i128::from(vx);
    nz >= 0
}

/// Draw a single scanline for flat shading with Z-buffering
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_flat(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    if let Some((fb_slice, zb_slice, mut z)) =
        prepare_scanline(fb, zb, y, x_start, x_end, z_start, dz_dx)
    {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                *pixel = color;
            }
            z += dz_dx;
        }
    }
}

/// Draw a single scanline for flat shading with Z-buffering and Alpha Blending
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_flat_blended(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    if let Some((fb_slice, zb_slice, mut z)) =
        prepare_scanline(fb, zb, y, x_start, x_end, z_start, dz_dx)
    {
        // Alpha blending parameters
        let alpha = (color >> 24) & 0xFF;
        let inv_alpha = 255 - alpha;

        // Optimization: Hoist source color unpacking and scaling out of the loop.
        // blend_swar(src, dest, alpha, inv_alpha) computes:
        // (src * inv_alpha + dest * alpha) >> 8
        // We precalculate (src * inv_alpha) here.
        let rb_src = (color & 0x00FF_00FF) * inv_alpha;
        let ag_src = ((color >> 8) & 0x00FF_00FF) * inv_alpha;

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Test Z but do not write Z for transparent pixels
            if z < *depth_val {
                let dest = *pixel;

                // Inline blend_swar logic with precalculated source term
                let rb_dest = dest & 0x00FF_00FF;
                let ag_dest = (dest >> 8) & 0x00FF_00FF;

                let rb = ((rb_src + rb_dest * alpha) >> 8) & 0x00FF_00FF;
                let ag = ((ag_src + ag_dest * alpha) >> 8) & 0x00FF_00FF;

                *pixel = rb | (ag << 8);
            }
            z += dz_dx;
        }
    }
}

/// Fill a 3D triangle with z-buffer test.
///
/// This function takes vertices in **Homogeneous Clip Space**.
/// It handles:
/// 1.  Frustum Clipping
/// 2.  Perspective Division (converting to Normalized Device Coordinates)
/// 3.  Viewport Mapping (converting to Screen Space)
/// 4.  Rasterization & Depth Testing
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices as `(Position, W)`. `Position` is `Vec3` (x,y,z).
/// *   `color` - 0xAARRGGBB color value.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_3d;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Define vertices in Clip Space (x, y, z, w)
/// // Visible range: -w <= x,y,z <= w
/// let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
/// let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
/// let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
/// let color = 0xFFFF0000; // Red
///
/// fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
/// ```
pub fn fill_triangle_3d(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    color: u32,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| (v.0, v.1));

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0, v0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0, v1.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0, v2.1, half_width, half_height);

        // Backface Culling (on original unsorted vertices)
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Sort by y
        let mut verts = [p0_orig, p1_orig, p2_orig];
        sort_by_y(&mut verts, |p| p.y);
        let [p0, p1, p2] = verts;

        // Prevent overflow when p2.y is i32::MAX and p0.y is i32::MIN
        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        // Optimization: Clamp Y range to screen bounds
        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        // Optimization: Pre-calculate dz/dx constant for the whole triangle
        // Plane equation: Ax + By + Cz + D = 0
        // vectors p0->p1 and p0->p2
        // Use i64 for coordinate differences to prevent overflow with extreme coordinates
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;

        // Cross product to get normal (A, B, C)
        let nx = uy * vz - uz * vy;
        // let ny = uz * vx - ux * vz;
        let nz = ux * vy - uy * vx; // This is actually 2D cross product of XY (area) of SORTED triangle

        // dz/dx = -A/C = -nx/nz
        let dz_dx = if nz.abs() > 0.0001 { -nx / nz } else { 0.0 };

        // Determine if long edge is on the left or right
        // Optimization: Use the sign of the cross product (nz) to determine winding
        // If nz > 0, p1 is to the right of p0->p2, so long edge (p0->p2) is Left.
        let long_edge_is_left = nz > 0.0;

        let mut edge_a = EdgeWalker::new(p0, p2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = EdgeWalker::new(p0, p1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = EdgeWalker::new(p1, p2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = EdgeWalker::new(p1, p2);
            }

            let (x_start, x_end, z_left) = if long_edge_is_left {
                ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z)
            } else {
                ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z)
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 {
                    // SAFETY: Safe due to clamps on x_start and y.
                    unsafe {
                        let alpha = (color >> 24) & 0xFF;
                        if alpha == 0xFF {
                            if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            }
                        } else {
                            // Transparent single pixel
                            let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                            if z_left < z_current {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, alpha, 255 - alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
                }
            } else {
                let alpha = (color >> 24) & 0xFF;
                if alpha == 0xFF {
                    draw_scanline_flat(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
                } else {
                    draw_scanline_flat_blended(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
                }
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to convert pre-scaled (0.0-255.0) Vec3 color to u32 ARGB
#[must_use]
#[inline(always)]
#[allow(clippy::missing_const_for_fn)]
fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
}

// Fixed point scale factor (16.16)
pub(crate) const FIXED_SCALE: f32 = 65536.0;

/// Draw a single scanline for Gouraud shading
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    c_start: (i64, i64, i64), // Fixed point color
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;

    // Use i64 for accumulators to prevent overflow when x_start is far off-screen
    let mut r_i = c_start.0;
    let mut g_i = c_start.1;
    let mut b_i = c_start.2;
    let (dr, dg, db) = (i64::from(dc_dx.0), i64::from(dc_dx.1), i64::from(dc_dx.2));

    // Clamp to screen bounds
    if xs < 0 {
        let diff = -i64::from(xs);
        z += (diff as f32) * dz_dx;
        let diff_i64 = diff;
        r_i += diff_i64 * dr;
        g_i += diff_i64 * dg;
        b_i += diff_i64 * db;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    // Optimization: Demote to i32 for the hot loop to reduce register pressure.
    // We used i64 above to handle large off-screen jumps safely without overflow.
    // Once on-screen, 16.16 fixed point color fits comfortably in i32.
    // (Max value ~255 * 65536 = 1.6e7 << i32::MAX)
    let mut r_i = r_i as i32;
    let mut g_i = g_i as i32;
    let mut b_i = b_i as i32;
    let dr = dr as i32;
    let dg = dg as i32;
    let db = db as i32;

    if xs <= xe {
        // Optimization: Use slice iterators to avoid index recalculation and bounds checks in the loop
        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (xs as usize);
        let end_idx = y_offset + (xe as usize);

        // SAFETY:
        // 1. xs and xe are clamped to [0, width-1] by the logic above.
        // 2. y is clamped to [0, height-1] by the caller (fill_triangle_gouraud).
        // 3. We checked `xs <= xe` immediately above, so `start_idx <= end_idx`.
        // Therefore, the range is valid and within bounds.
        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        // Optimization: Check for fast path (no clamping needed)
        // If all color channels are within [0, 255] for the entire span, we can skip clamping.
        // r_i is 16.16 fixed point. Max value is 255.0 = 0x00FF_0000.
        // We calculate end values based on start + delta * count.
        let count = xe - xs;
        let r_end = r_i.wrapping_add(dr.wrapping_mul(count));
        let g_end = g_i.wrapping_add(dg.wrapping_mul(count));
        let b_end = b_i.wrapping_add(db.wrapping_mul(count));

        // Use strict upper bound 0x0100_0000 (256.0) to ensure integer part fits in u8.
        // Cast to u32 handles negative check (becomes large u32).
        let safe_limit: u32 = 0x0100_0000;
        let safe = (r_i as u32) < safe_limit
            && (r_end as u32) < safe_limit
            && (g_i as u32) < safe_limit
            && (g_end as u32) < safe_limit
            && (b_i as u32) < safe_limit
            && (b_end as u32) < safe_limit;

        if safe {
            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                if z < *depth_val {
                    *depth_val = z;
                    // Fast path: direct shift, no clamp/mask
                    // r_i as u32 >> 16 extracts the integer part (0..255)
                    let r = (r_i as u32) >> 16;
                    let g = (g_i as u32) >> 16;
                    let b = (b_i as u32) >> 16;

                    *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
                z += dz_dx;
                r_i += dr;
                g_i += dg;
                b_i += db;
            }
        } else {
            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                // Check depth buffer
                if z < *depth_val {
                    *depth_val = z;
                    // Unpack fixed point color
                    // Optimization: Combine clamp and mask to avoid shifts and intermediate u8 casts
                    // 16.16 fixed point means 255.0 is 0x00FF0000
                    let r = r_i.clamp(0, 0x00FF_0000);
                    let g = g_i.clamp(0, 0x00FF_0000);
                    let b = b_i.clamp(0, 0x00FF_0000);

                    *pixel = 0xFF00_0000
                        | ((r as u32) & 0x00FF_0000)
                        | (((g as u32) & 0x00FF_0000) >> 8)
                        | (((b as u32) & 0x00FF_0000) >> 16);
                }
                z += dz_dx;
                r_i += dr;
                g_i += dg;
                b_i += db;
            }
        }
    }
}

/// Helper to iterate along the edge of a triangle in screen space.
///
/// This struct manages the state for walking down a triangle edge, interpolating
/// X and Z coordinates. It uses fixed-point arithmetic for X to ensure
/// pixel-perfect rasterization consistency.
pub(crate) struct EdgeWalker {
    /// Current X coordinate in 16.16 fixed-point format.
    ///
    /// The upper 16 bits represent the integer pixel coordinate.
    /// The lower 16 bits represent sub-pixel precision.
    pub(crate) x: i64,
    /// Change in X per scanline (dx/dy) in 16.16 fixed-point format.
    dx_dy: i64,
    /// Current Z depth.
    pub(crate) z: f32,
    /// Change in Z per scanline (dz/dy).
    dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0.0)
        } else {
            let inv_h = 1.0 / height;

            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
    }
}

struct GouraudGradients {
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
}

impl GouraudGradients {
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vc = c2 - c0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_r = uy * vc.x - uc.x * vy;
        let nx_g = uy * vc.y - uc.y * vy;
        let nx_b = uy * vc.z - uc.z * vy;

        let dr = nx_r * inv_nz;
        let dg = nx_g * inv_nz;
        let db = nx_b * inv_nz;

        let dr_i = (dr * FIXED_SCALE) as i32;
        let dg_i = (dg * FIXED_SCALE) as i32;
        let db_i = (db * FIXED_SCALE) as i32;

        Self {
            dz_dx,
            dc_dx: (dr_i, dg_i, db_i),
        }
    }

    fn is_long_edge_left(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        ux * vy - uy * vx > 0.0
    }
}

struct GouraudEdgeWalker {
    x: i64,
    z: f32,
    c: (i64, i64, i64),
    dx_dy: i64,
    dz_dy: f32,
    dc_dy: (i64, i64, i64),
}

impl GouraudEdgeWalker {
    fn new(p_start: ScreenPoint, p_end: ScreenPoint, c_start: Vec3, c_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height == 0.0 {
            (0, 0.0, (0, 0, 0))
        } else {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            c: c_fixed,
            dx_dy,
            dz_dy,
            dc_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.c.0 += self.dc_dy.0;
        self.c.1 += self.dc_dy.1;
        self.c.2 += self.dc_dy.2;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.c.0 = self.c.0.wrapping_add(self.dc_dy.0.wrapping_mul(n));
        self.c.1 = self.c.1.wrapping_add(self.dc_dy.1.wrapping_mul(n));
        self.c.2 = self.c.2.wrapping_add(self.dc_dy.2.wrapping_mul(n));
    }
}

/// Fill a 3D triangle with Gouraud (per-vertex) shading.
///
/// This function linearly interpolates colors across the face of the triangle.
/// Lighting calculations are performed at vertices, and the resulting colors
/// are passed to this function.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Color)`.
///     *   `Position`: Clip Space position.
///     *   `W`: Homogeneous W coordinate.
///     *   `Color`: RGB color (0.0 - 1.0) for the vertex.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_gouraud;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Vertices: ((x, y, z, w), (r, g, b))
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec3::new(1.0, 0.0, 0.0)); // Red
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 1.0, 0.0)); // Green
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0)); // Blue
///
/// fill_triangle_gouraud(&mut fb, &mut zb, v0, v1, v2);
/// ```
pub fn fill_triangle_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3), // ((position, w), color)
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Optimization: Pre-scale colors to 0..255 for faster interpolation and packing
        // allowing us to skip clamp/mul per pixel
        let c0 = v0.1 * 255.0;
        let c1 = v1.1 * 255.0;
        let c2 = v2.1 * 255.0;

        // Sort by y
        let mut verts = [(p0_orig, c0), (p1_orig, c1), (p2_orig, c2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, c0), (p1, c1), (p2, c2)] = verts;

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

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = {
            let g = GouraudGradients::new(p0, p1, p2, c0, c1, c2);
            let left = GouraudGradients::is_long_edge_left(p0, p1, p2);
            (g, left)
        };

        let mut edge_a = GouraudEdgeWalker::new(p0, p2, c0, c2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
            }

            let (x_start, x_end, z_left, c_left) = if long_edge_is_left {
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

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                            fb.set_pixel_unchecked(
                                x_start as usize,
                                y as usize,
                                pack_color_fixed(c_left),
                            );
                        }
                    }
                }
            } else {
                draw_scanline_gouraud(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    z_left,
                    c_left,
                    gradients.dz_dx,
                    gradients.dc_dx,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Fill a 3D triangle with custom lighting
#[allow(clippy::too_many_arguments)] // Rendering API requires all parameters explicitly
pub fn fill_triangle_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
    ambient_color: Vec3,
    light_dir: Vec3, // Direction the light travels
    light_color: Vec3,
) {
    // Ambient shade
    let ambient = base_color * ambient_color;

    // Diffuse shade: base * light * max(0, normal . -dir)
    let intensity = normal.dot(light_dir * -1.0).max(0.0);
    let diffuse = base_color * light_color * intensity;

    // Combine and clamp (clamping handled by color_to_u32)
    let final_color = ambient + diffuse;

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

#[derive(Clone, Copy)]
pub struct PerspectiveTextureGradients {
    pub dz_dx: f32,
    pub dq_dx: f32,
    pub du_dx: f32,
    pub dv_dx: f32,
    pub dq_dy: f32,
    pub du_dy: f32,
    pub dv_dy: f32,
}

impl PerspectiveTextureGradients {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        // Calculate Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }
}

pub(crate) struct PerspectiveTextureEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PerspectiveSpanStart {
    pub(crate) z: f32,
    pub(crate) q: f32,
    pub(crate) u: f32,
    pub(crate) v: f32,
}

pub(crate) const RECIPROCAL_TABLE: [f32; 17] = [
    0.0,
    1.0,
    0.5,
    0.333_333_34,
    0.25,
    0.2,
    0.166_666_67,
    0.142_857_15,
    0.125,
    0.111_111_11,
    0.1,
    0.090_909_09,
    0.083_333_336,
    0.076_923_08,
    0.071_428_575,
    0.066_666_67,
    0.062_5,
];

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_nearest(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    // Hoist texture properties
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let tex_w_usize = tex_w as usize;

    let shift = texture.width_shift;
    // Optimization: Loop versioning.
    // Duplicate the loop to specialize for power-of-two textures.
    // This hoists the branch `if shift < 32` out of the tight loop and allows
    // the use of bitwise shifting `v << shift` instead of multiplication `v * width`.
    if shift < 32 {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                // Inline sampling
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[((v as usize) << shift) + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, alpha, 255 - alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    } else {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                // Inline sampling
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[(v as usize) * tex_w_usize + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, alpha, 255 - alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_bilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let shift = texture.width_shift;

    let w_i32 = (tex_w as i32).wrapping_sub(1);
    let h_i32 = (tex_h as i32).wrapping_sub(1);
    let tex_w_usize = tex_w as usize;

    // Optimization: Use a macro to hoist the `shift < 32` check out of the hot loop.
    // This allows the compiler to generate two specialized versions of the loop:
    // one using bit-shifting (fast) and one using multiplication (slower),
    // without branching inside the loop for every pixel.
    macro_rules! process_span_bilinear {
        ($op:tt, $val:expr) => {
            let mut cached_x0 = i32::MIN;
            let mut cached_y0 = i32::MIN;
            let mut c00 = 0;
            let mut c10 = 0;
            let mut c01 = 0;
            let mut c11 = 0;

            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                if z < *depth_val {
                    let u_img_fixed = u_fix >> 8;
                    let v_img_fixed = v_fix >> 8;

                    let x0_raw = u_img_fixed >> 8;
                    let y0_raw = v_img_fixed >> 8;

                    if x0_raw != cached_x0 || y0_raw != cached_y0 {
                        cached_x0 = x0_raw;
                        cached_y0 = y0_raw;

                        let (t00, t10, t01, t11) =
                            if (x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32) {
                                let x0 = x0_raw as usize;
                                let y0 = y0_raw as usize;

                                let row0 = y0 $op $val;
                                let row1 = row0 + tex_w_usize;

                                unsafe {
                                    // Optimization: Read 2 pixels at a time as u64.
                                    #[cfg(target_endian = "little")]
                                    {
                                        let ptr = tex_pixels.as_ptr();
                                        let row0_pair = ptr.add(row0 + x0).cast::<u64>().read_unaligned();
                                        let row1_pair = ptr.add(row1 + x0).cast::<u64>().read_unaligned();

                                        (
                                            row0_pair as u32,
                                            (row0_pair >> 32) as u32,
                                            row1_pair as u32,
                                            (row1_pair >> 32) as u32,
                                        )
                                    }
                                    #[cfg(not(target_endian = "little"))]
                                    {
                                        (
                                            *tex_pixels.get_unchecked(row0 + x0),
                                            *tex_pixels.get_unchecked(row0 + x0 + 1),
                                            *tex_pixels.get_unchecked(row1 + x0),
                                            *tex_pixels.get_unchecked(row1 + x0 + 1),
                                        )
                                    }
                                }
                            } else {
                                let x0 = x0_raw.clamp(0, w_i32) as usize;
                                let y0 = y0_raw.clamp(0, h_i32) as usize;
                                let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
                                let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

                                let row0 = y0 $op $val;
                                let row1 = y1 $op $val;

                                unsafe {
                                    (
                                        *tex_pixels.get_unchecked(row0 + x0),
                                        *tex_pixels.get_unchecked(row0 + x1),
                                        *tex_pixels.get_unchecked(row1 + x0),
                                        *tex_pixels.get_unchecked(row1 + x1),
                                    )
                                }
                            };
                        c00 = t00;
                        c10 = t10;
                        c01 = t01;
                        c11 = t11;
                    }

                    let wx = (u_img_fixed & 0xFF) as u32;
                    let wy = (v_img_fixed & 0xFF) as u32;

                    let final_color = blend_four_way(c00, c10, c01, c11, wx, wy);

                    let alpha = (final_color >> 24) & 0xFF;
                    if alpha == 255 {
                        *depth_val = z;
                        *pixel = final_color;
                    } else if alpha > 0 {
                        let dest = *pixel;
                        *pixel = blend_swar(final_color, dest, alpha, 255 - alpha);
                    }
                }
                z += dz_dx;
                u_fix = u_fix.wrapping_add(du_fix);
                v_fix = v_fix.wrapping_add(dv_fix);
            }
        };
    }

    if shift < 32 {
        process_span_bilinear!(<<, shift);
    } else {
        process_span_bilinear!(*, tex_w_usize);
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_trilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
    lod: f32,
) {
    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            let color = texture.get_pixel_trilinear_fixed(u_fix, v_fix, lod);
            let alpha = (color >> 24) & 0xFF;

            if alpha == 255 {
                *depth_val = z;
                *pixel = color;
            } else if alpha > 0 {
                let dest = *pixel;
                *pixel = blend_swar(color, dest, alpha, 255 - alpha);
            }
        }
        z += dz_dx;
        u_fix = u_fix.wrapping_add(du_fix);
        v_fix = v_fix.wrapping_add(dv_fix);
    }
}

/// Draw a single scanline with perspective-correct texture mapping
/// Optimized using span-based interpolation (every 16 pixels)
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_textured_perspective(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    texture: &Texture,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PerspectiveSpanStart,
    gradients: &PerspectiveTextureGradients,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let span_size = 16;
    let mut x = xs;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while x <= xe {
        let remaining = xe - x + 1;
        let count = remaining.min(span_size);

        // End values at 'x + count'
        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        // Perform perspective divide at span endpoints
        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        // Interpolate texel coordinates linearly over the span
        // Optimization: Use reciprocal table to replace division with multiplication
        // count is guaranteed to be in [1, 16]
        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (x as usize);
        let end_idx = y_offset + ((x + count - 1) as usize);

        // SAFETY: Bounds checked by xs, xe clamping and loop logic
        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        match texture.filter_mode {
            FilterMode::Nearest => {
                // Fixed point optimization for Nearest Neighbor
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_nearest(
                    fb_slice,
                    zb_slice,
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
                // Fixed point optimization for Bilinear
                // Use 16.16 for accumulation to maintain precision, then downshift to 24.8 for sampling
                // Optimization: Subtract 0.5 (128 units in 24.8, 32768 in 16.16) upfront
                // to avoid per-pixel subtraction in get_pixel_bilinear_fixed
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_bilinear(
                    fb_slice,
                    zb_slice,
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
                // For Trilinear, we need LOD.
                // Calculate LOD at span start to avoid extra per-pixel work.
                let w = w_start; // 1/q
                let w_sq = w * w;

                // Derivatives of texture coordinates with respect to screen x/y.
                // u_tex = u / q, v_tex = v / q.
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

                draw_span_trilinear(
                    fb_slice,
                    zb_slice,
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

        // Advance state
        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;

        // Reuse end values for next start
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;

        x += count;
    }
}
/// Fill a 3D triangle with texture mapping.
///
/// This function performs perspective-correct texture mapping using standard scanline rasterization.
/// It interpolates texture coordinates ($u, v$) and perspective term ($1/w$) across the triangle surface.
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices, each defined as `((Position, W), UV)`.
///     *   `Position`: 3D vertex position in Clip Space (before perspective divide).
///     *   `W`: Homogeneous W coordinate (distance from camera plane).
///     *   `UV`: Texture coordinates in range $[0.0, 1.0]$.
/// *   `texture` - The source texture to map onto the triangle.
///
/// # Perspective Correction
///
/// To avoid texture swimming (warping) when viewing triangles at an angle, this rasterizer
/// performs perspective-correct interpolation:
/// 1.  At each vertex, calculate $q = 1/w$, $u' = u/w$, $v' = v/w$.
/// 2.  Linearly interpolate $q, u', v'$ across the screen.
/// 3.  Per-pixel (or per-span), recover $u = u'/q$ and $v = v'/q$.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_textured;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::texture::Texture;
/// use abrash::math::{Vec3, Vec2};
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Create a simple checkerboard texture
/// let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();
///
/// // Define vertices in Clip Space ((Position, W), UV)
/// // Triangle covering center of screen
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec2::new(0.5, 0.0));
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec2::new(0.0, 1.0));
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec2::new(1.0, 1.0));
///
/// fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
///
/// // Verify center pixel was drawn
/// assert_ne!(fb.get_pixel(50, 50), Some(0x00000000));
/// ```
pub fn fill_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2),
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        let ux_orig = (i64::from(p1_orig.x) - i64::from(p0_orig.x)) as f32;
        let uy_orig = (i64::from(p1_orig.y) - i64::from(p0_orig.y)) as f32;
        let vx_orig = (i64::from(p2_orig.x) - i64::from(p0_orig.x)) as f32;
        let vy_orig = (i64::from(p2_orig.y) - i64::from(p0_orig.y)) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
            continue;
        }

        // Prepare perspective attributes: q=1/w, u/w, v/w
        // Note: We multiply UV by texture dimensions here so interpolation happens in texel space

        // Optimization: Reuse inv_w calculated during projection to avoid division
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let u0 = v0.1.x * texture.width as f32 * inv_w0;
        let v0_val = v0.1.y * texture.height as f32 * inv_w0;

        let u1 = v1.1.x * texture.width as f32 * inv_w1;
        let v1_val = v1.1.y * texture.height as f32 * inv_w1;

        let u2 = v2.1.x * texture.width as f32 * inv_w2;
        let v2_val = v2.1.y * texture.height as f32 * inv_w2;

        // Sort by y
        // We need to keep track of all attributes (p, u, v) - q is inside p
        let mut verts = [
            (p0_orig, u0, v0_val),
            (p1_orig, u1, v1_val),
            (p2_orig, u2, v2_val),
        ];
        sort_by_y(&mut verts, |(p, _, _)| p.y);
        let [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

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

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = {
            let g =
                PerspectiveTextureGradients::new(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2);

            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;

            (g, left)
        };

        let mut edge_a = PerspectiveTextureEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PerspectiveTextureEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            }

            let (x_start, x_end, z_left, q_left, u_left, v_left) = if long_edge_is_left {
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

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 && q_left.abs() > 0.000_001 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                        if z_left < z_current {
                            let w = 1.0 / q_left;
                            let u_tex = u_left * w;
                            let v_tex = v_left * w;
                            let color = match texture.filter_mode {
                                FilterMode::Nearest => {
                                    texture.get_pixel_texel(u_tex as i32, v_tex as i32)
                                }
                                FilterMode::Bilinear => {
                                    texture.get_pixel_bilinear_texel(u_tex, v_tex)
                                }
                                FilterMode::Trilinear => {
                                    // Calculate LOD for single pixel
                                    // q = 1/w.
                                    // u_tex = u/q.
                                    // du_tex/dx = (du/dx * q - u * dq/dx) / q^2
                                    let w = 1.0 / q_left;
                                    let w_sq = w * w;

                                    let du_tex_dx = (gradients.du_dx * q_left
                                        - u_left * gradients.dq_dx)
                                        * w_sq;
                                    let dv_tex_dx = (gradients.dv_dx * q_left
                                        - v_left * gradients.dq_dx)
                                        * w_sq;
                                    let du_tex_dy = (gradients.du_dy * q_left
                                        - u_left * gradients.dq_dy)
                                        * w_sq;
                                    let dv_tex_dy = (gradients.dv_dy * q_left
                                        - v_left * gradients.dq_dy)
                                        * w_sq;

                                    let max_rho_sq = (du_tex_dx * du_tex_dx
                                        + dv_tex_dx * dv_tex_dx)
                                        .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                                    let lod = 0.5 * max_rho_sq.log2();
                                    texture.get_pixel_trilinear(u_tex, v_tex, lod)
                                }
                            };

                            let alpha = (color >> 24) & 0xFF;
                            if alpha == 255 {
                                // Manually update Z
                                let width_usize = fb.width() as usize;
                                let idx = (y as usize) * width_usize + (x_start as usize);
                                *zb.as_mut_slice().get_unchecked_mut(idx) = z_left;
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            } else if alpha > 0 {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, alpha, 255 - alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
                }
            } else {
                draw_scanline_textured_perspective(
                    fb,
                    zb,
                    texture,
                    y,
                    x_start,
                    x_end,
                    PerspectiveSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                    },
                    &gradients,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[derive(Clone, Copy)]
struct PhongGradients {
    dz_dx: f32,
    dnx_dx: f32, // d(nx/w)/dx
    dny_dx: f32,
    dnz_dx: f32,
}

impl PhongGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;

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

        Self {
            dz_dx,
            dnx_dx,
            dny_dx,
            dnz_dx,
        }
    }
}

struct PhongEdgeWalker {
    x: i64,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
}

impl PhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(p_start: ScreenPoint, p_end: ScreenPoint, n_start: Vec3, n_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            dx_dy,
            dz_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct PhongSpanStart {
    z: f32,
    // q unused in optimization
    nx: f32,
    ny: f32,
    nz: f32,
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    target_feature = "avx2"
))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_phong_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    mut z: f32,
    mut nx: f32,
    mut ny: f32,
    mut nz: f32,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled by 255.0
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled by 255.0
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    // Load constants
    let dz_dx_vec = _mm256_set1_ps(gradients.dz_dx);
    let dnx_dx_vec = _mm256_set1_ps(gradients.dnx_dx);
    let dny_dx_vec = _mm256_set1_ps(gradients.dny_dx);
    let dnz_dx_vec = _mm256_set1_ps(gradients.dnz_dx);

    let lx = _mm256_set1_ps(neg_light_dir.x);
    let ly = _mm256_set1_ps(neg_light_dir.y);
    let lz = _mm256_set1_ps(neg_light_dir.z);

    let diff_r = _mm256_set1_ps(pre_diffuse_255.x);
    let diff_g = _mm256_set1_ps(pre_diffuse_255.y);
    let diff_b = _mm256_set1_ps(pre_diffuse_255.z);

    let amb_r = _mm256_set1_ps(ambient_255.x);
    let amb_g = _mm256_set1_ps(ambient_255.y);
    let amb_b = _mm256_set1_ps(ambient_255.z);

    let epsilon = _mm256_set1_ps(0.0001);
    let zero = _mm256_setzero_ps();
    let one = _mm256_set1_ps(1.0);
    let one_point_five = _mm256_set1_ps(1.5);
    let zero_point_five = _mm256_set1_ps(0.5);
    let scale_255 = _mm256_set1_ps(255.0);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

    // Initial offsets for 8 pixels
    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx_vec, offsets));
    let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(nx), _mm256_mul_ps(dnx_dx_vec, offsets));
    let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(ny), _mm256_mul_ps(dny_dx_vec, offsets));
    let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(nz), _mm256_mul_ps(dnz_dx_vec, offsets));

    // Steps for 8 pixels
    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
    let dnx_step = _mm256_mul_ps(dnx_dx_vec, _mm256_set1_ps(8.0));
    let dny_step = _mm256_mul_ps(dny_dx_vec, _mm256_set1_ps(8.0));
    let dnz_step = _mm256_mul_ps(dnz_dx_vec, _mm256_set1_ps(8.0));

    while i + 8 <= len {
        // Load depth buffer
        let depth_ptr = zb_slice.as_mut_ptr().add(i);
        let depth_val = _mm256_loadu_ps(depth_ptr);

        // Z-test
        let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
        let mask_int = _mm256_castps_si256(mask);

        // If any pixel passes Z-test
        if _mm256_movemask_ps(mask) != 0 {
            // Update Z-buffer
            // Optimization: Use load-blend-store instead of maskstore which can be slow
            let old_z = _mm256_loadu_ps(depth_ptr);
            let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
            _mm256_storeu_ps(depth_ptr, new_z);

            // Shading
            let nx_sq = _mm256_mul_ps(nx_vec, nx_vec);
            let ny_sq = _mm256_mul_ps(ny_vec, ny_vec);
            let nz_sq = _mm256_mul_ps(nz_vec, nz_vec);
            let len_sq = _mm256_add_ps(nx_sq, _mm256_add_ps(ny_sq, nz_sq));

            // Check if len_sq > epsilon
            let len_valid = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);

            // Calculate rsqrt. Avoid rsqrt(0) by blending with 1.0 (doesn't matter what value, masked out later)
            let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
            let rsqrt = _mm256_rsqrt_ps(safe_len_sq);

            // Newton-Raphson iteration: y = y * (1.5 - 0.5 * x * y * y)
            let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
            let iter2 = _mm256_sub_ps(one_point_five, _mm256_mul_ps(zero_point_five, iter1));
            let inv_len = _mm256_mul_ps(rsqrt, iter2);

            // Dot product (unnormalized)
            let dot_x = _mm256_mul_ps(nx_vec, lx);
            let dot_y = _mm256_mul_ps(ny_vec, ly);
            let dot_z = _mm256_mul_ps(nz_vec, lz);
            let dot_unorm = _mm256_add_ps(dot_x, _mm256_add_ps(dot_y, dot_z));

            // Intensity
            let intensity_raw = _mm256_mul_ps(dot_unorm, inv_len);
            let intensity = _mm256_max_ps(zero, intensity_raw);

            // Apply mask for valid length
            let intensity = _mm256_blendv_ps(zero, intensity, len_valid);

            // Calculate Color (Pre-scaled)
            let r = _mm256_add_ps(amb_r, _mm256_mul_ps(diff_r, intensity));
            let g = _mm256_add_ps(amb_g, _mm256_mul_ps(diff_g, intensity));
            let b = _mm256_add_ps(amb_b, _mm256_mul_ps(diff_b, intensity));

            // Clamp and convert to u32
            // Clamp 0.0-255.0
            let r_clamp = _mm256_min_ps(_mm256_max_ps(r, zero), scale_255);
            let g_clamp = _mm256_min_ps(_mm256_max_ps(g, zero), scale_255);
            let b_clamp = _mm256_min_ps(_mm256_max_ps(b, zero), scale_255);

            // Already scaled
            let r_255 = r_clamp;
            let g_255 = g_clamp;
            let b_255 = b_clamp;

            // Convert to i32 (truncation match scalar?) Scalar uses `as u32` which is truncation.
            // cvttps truncates.
            let r_i = _mm256_cvttps_epi32(r_255);
            let g_i = _mm256_cvttps_epi32(g_255);
            let b_i = _mm256_cvttps_epi32(b_255);

            // Pack: 0xFF000000 | (r << 16) | (g << 8) | b
            let pixel_val = _mm256_or_si256(
                alpha_mask,
                _mm256_or_si256(
                    _mm256_slli_epi32(r_i, 16),
                    _mm256_or_si256(_mm256_slli_epi32(g_i, 8), b_i),
                ),
            );

            // Store pixels
            let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
            let old_color = _mm256_loadu_si256(fb_ptr);
            // blendv_epi8 blends based on the high bit of each byte.
            // Our mask is 32-bit 0xFFFFFFFF or 0x00000000, so it works for bytes too.
            let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
            _mm256_storeu_si256(fb_ptr, new_color);
        }

        // Advance
        z_vec = _mm256_add_ps(z_vec, dz_step);
        nx_vec = _mm256_add_ps(nx_vec, dnx_step);
        ny_vec = _mm256_add_ps(ny_vec, dny_step);
        nz_vec = _mm256_add_ps(nz_vec, dnz_step);

        i += 8;
    }

    // Scalar tail loop
    while i < len {
        // We need to extract current scalar values from vector state or recompute?
        // Recomputing is safer/easier than extraction.
        // Or simply maintain scalar counters parallel to vector?
        // But vector state is already advanced.

        // Let's just recompute for the tail from the current `i`.
        // x_current = x_start + i
        // value = start + gradient * i

        let i_f = i as f32;
        let mut z = z + i_f * gradients.dz_dx;
        let mut nx = nx + i_f * gradients.dnx_dx;
        let mut ny = ny + i_f * gradients.dny_dx;
        let mut nz = nz + i_f * gradients.dnz_dx;

        let pixel = &mut fb_slice[i];
        let depth_val = &mut zb_slice[i];

        if z < *depth_val {
            *depth_val = z;

            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        i += 1;
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PhongSpanStart,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
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

    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "avx2"
    ))]
    if is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_phong_simd(
                fb_slice,
                zb_slice,
                z,
                nx,
                ny,
                nz,
                gradients,
                pre_diffuse_255,
                neg_light_dir,
                ambient_255,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Optimization: Deferred Normalization.
            // Instead of constructing a Vec3 and calling fast_normalize() (which does len_sq, inv_sqrt, and 3 muls),
            // we compute len_sq and the unnormalized dot product first.
            // intensity = dot(N_norm, L) = dot(N / |N|, L) = dot(N, L) / |N| = dot(N, L) * fast_inv_sqrt(|N|^2)
            // This saves 2 multiplications per pixel and avoids Vec3 construction overhead.
            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        z += gradients.dz_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
    }
}

/// Fill a 3D triangle with Phong Shading (per-pixel lighting).
///
/// This function interpolates the normal vector across the triangle surface
/// and computes the Blinn-Phong lighting equation at every pixel.
/// It produces much smoother highlights than Gouraud shading but is more computationally expensive.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Normal)`.
///     *   `Normal`: Surface normal vector at the vertex.
/// *   `color`: Base diffuse color of the material.
/// *   `light_dir`: Direction *to* the light source (normalized).
/// *   `light_color`: Color/Intensity of the light.
/// *   `ambient`: Ambient light color.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_phong;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Vertices with normals
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
///
/// let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
///
/// fill_triangle_phong(
///     &mut fb, &mut zb,
///     v0, v1, v2,
///     Vec3::new(1.0, 0.0, 0.0), // Red material
///     light_dir,
///     Vec3::new(1.0, 1.0, 1.0), // White light
///     Vec3::new(0.1, 0.1, 0.1), // Dim ambient
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3),
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
    color: Vec3,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes: q=1/w, n/w
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        let mut verts = [(p0_orig, n0), (p1_orig, n1), (p2_orig, n2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, n0), (p1, n1), (p2, n2)] = verts;

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

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = {
            let g = PhongGradients::new(p0, p1, p2, n0, n1, n2);
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a = PhongEdgeWalker::new(p0, p2, n0, n2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PhongEdgeWalker::new(p0, p1, n0, n1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PhongEdgeWalker::new(p1, p2, n1, n2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Precalculate lighting constants
        // Optimization: Pre-scale by 255.0 to avoid per-pixel multiplication
        let pre_diffuse_255 = color * light_color * 255.0;
        let neg_light_dir = light_dir * -1.0;
        let ambient_255 = ambient * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PhongEdgeWalker::new(p1, p2, n1, n2);
            }

            let (x_start, x_end, z_left, nx_left, ny_left, nz_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_phong(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    PhongSpanStart {
                        z: z_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                    },
                    &gradients,
                    pre_diffuse_255,
                    neg_light_dir,
                    ambient_255,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[derive(Clone, Copy)]
struct NormalMapGradients {
    dz_dx: f32,
    dq_dx: f32,  // 1/w
    du_dx: f32,  // u/w
    dv_dx: f32,  // v/w
    dnx_dx: f32, // nx/w
    dny_dx: f32,
    dnz_dx: f32,
    dtx_dx: f32, // tx/w
    dty_dx: f32,
    dtz_dx: f32,
    dtw_dx: f32, // tw/w
}

impl NormalMapGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        t0: Vec4,
        t1: Vec4,
        t2: Vec4,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let utx = t1.x - t0.x;
        let uty = t1.y - t0.y;
        let utz = t1.z - t0.z;
        let utw = t1.w - t0.w;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vtx = t2.x - t0.x;
        let vty = t2.y - t0.y;
        let vtz = t2.z - t0.z;
        let vtw = t2.w - t0.w;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_tx = uy * vtx - utx * vy;
        let dtx_dx = nx_tx * inv_nz;

        let nx_ty = uy * vty - uty * vy;
        let dty_dx = nx_ty * inv_nz;

        let nx_tz = uy * vtz - utz * vy;
        let dtz_dx = nx_tz * inv_nz;

        let nx_tw = uy * vtw - utw * vy;
        let dtw_dx = nx_tw * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dnx_dx,
            dny_dx,
            dnz_dx,
            dtx_dx,
            dty_dx,
            dtz_dx,
            dtw_dx,
        }
    }
}

struct NormalMapEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    tx: f32,
    ty: f32,
    tz: f32,
    tw: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dtx_dy: f32,
    dty_dy: f32,
    dtz_dy: f32,
    dtw_dy: f32,
}

impl NormalMapEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        n_start: Vec3,
        n_end: Vec3,
        t_start: Vec4,
        t_end: Vec4,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dtx_dy = (t_end.x - t_start.x) * inv_h;
        let dty_dy = (t_end.y - t_start.y) * inv_h;
        let dtz_dy = (t_end.z - t_start.z) * inv_h;
        let dtw_dy = (t_end.w - t_start.w) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            tx: t_start.x,
            ty: t_start.y,
            tz: t_start.z,
            tw: t_start.w,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dtx_dy,
            dty_dy,
            dtz_dy,
            dtw_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.tx += self.dtx_dy;
        self.ty += self.dty_dy;
        self.tz += self.dtz_dy;
        self.tw += self.dtw_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.tx += self.dtx_dy * n_f;
        self.ty += self.dty_dy * n_f;
        self.tz += self.dtz_dy * n_f;
        self.tw += self.dtw_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct NormalMapSpanStart {
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    tx: f32,
    ty: f32,
    tz: f32,
    tw: f32,
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: NormalMapSpanStart,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    neg_light_dir: Vec3,
    pre_diffuse_color: Vec3, // base_color * light_color
    ambient: Vec3,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    // Local accumulators
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut tx = start.tx;
    let mut ty = start.ty;
    let mut tz = start.tz;
    let mut tw = start.tw;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        tx += diff_f * gradients.dtx_dx;
        ty += diff_f * gradients.dty_dx;
        tz += diff_f * gradients.dtz_dx;
        tw += diff_f * gradients.dtw_dx;
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

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Perspective recover
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let u_tex = u * w_recip;
            let v_tex = v * w_recip;

            // Sample diffuse
            // Using Nearest for speed in this complex shader, or duplicate logic for Bilinear
            let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack diffuse to Vec3 (0-1)
            let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
            let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
            let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
            let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

            // Sample normal map
            let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack to [-1, 1]
            let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let tangent_normal = Vec3::new(nm_r, nm_g, nm_b); // Usually Z is up in tangent space

            // TBN Construction
            // Normalize interpolated N and T (they are n/w and t/w, but direction is same)
            // Need to recover true direction
            let n_interp = Vec3::new(nx, ny, nz).normalize(); // Assuming non-zero
            let t_interp = Vec3::new(tx, ty, tz).normalize();

            // Gram-Schmidt re-orthogonalize T with respect to N
            let t_ortho = (t_interp - n_interp * n_interp.dot(t_interp)).normalize();

            // Calculate Bitangent
            // tw holds handedness * w. But we want just handedness.
            // w is 1/q. So tw/q = handedness * w / (1/w) = handedness * w^2? No.
            // tw is (handedness * 1.0) / w.
            // q is 1/w.
            // tw / q = handedness.
            let handedness = if (tw * w_recip) > 0.0 { 1.0 } else { -1.0 };
            let b_ortho = n_interp.cross(t_ortho) * handedness;

            // Transform normal from tangent space to world space
            // N_world = T * nm.x + B * nm.y + N * nm.z
            let final_normal = (t_ortho * tangent_normal.x
                + b_ortho * tangent_normal.y
                + n_interp * tangent_normal.z)
                .normalize();

            // Lighting
            let intensity = final_normal.dot(neg_light_dir).max(0.0);

            // Combine
            let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
            let final_color_vec = ambient + diffuse_total;
            *pixel = color_to_u32(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        u += gradients.du_dx;
        v += gradients.dv_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        tx += gradients.dtx_dx;
        ty += gradients.dty_dx;
        tz += gradients.dtz_dx;
        tw += gradients.dtw_dx;
    }
}

/// Fill a 3D triangle with Normal Mapping.
///
/// This function performs per-pixel tangent-space normal mapping.
///
/// # Arguments
///
/// * `v0`, `v1`, `v2` - Vertices defined as `((Position, W), UV, Normal, Tangent)`.
///   - `Tangent`: `Vec4` where xyz is the tangent vector and w is the handedness (+1/-1).
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2, Vec3, Vec4),
    v1: ((Vec3, f32), Vec2, Vec3, Vec4),
    v2: ((Vec3, f32), Vec2, Vec3, Vec4),
    texture: &Texture,
    normal_map: &Texture,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Scale UV by texture size (assuming both maps match size or using one size for ratio)
        // Usually, UVs are 0..1, we multiply by size to get texel coords
        let w = texture.width as f32;
        let h = texture.height as f32;

        let u0 = v0.1.x * w * inv_w0;
        let v0_val = v0.1.y * h * inv_w0;
        let u1 = v1.1.x * w * inv_w1;
        let v1_val = v1.1.y * h * inv_w1;
        let u2 = v2.1.x * w * inv_w2;
        let v2_val = v2.1.y * h * inv_w2;

        let n0 = v0.2 * inv_w0;
        let n1 = v1.2 * inv_w1;
        let n2 = v2.2 * inv_w2;

        let t0 = v0.3 * inv_w0;
        let t1 = v1.3 * inv_w1;
        let t2 = v2.3 * inv_w2;

        let mut verts = [
            (p0_orig, u0, v0_val, n0, t0),
            (p1_orig, u1, v1_val, n1, t1),
            (p2_orig, u2, v2_val, n2, t2),
        ];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [
            (p0, u0, v0_v, n0, t0),
            (p1, u1, v1_v, n1, t1),
            (p2, u2, v2_v, n2, t2),
        ] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

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

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = {
            let g = NormalMapGradients::new(
                p0, p1, p2, q0, q1, q2, u0, u1, u2, v0_v, v1_v, v2_v, n0, n1, n2, t0, t1, t2,
            );
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a =
            NormalMapEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0_v, v2_v, n0, n2, t0, t2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e =
                NormalMapEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0_v, v1_v, n0, n1, t0, t1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e =
                NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, n1, n2, t1, t2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let neg_light_dir = light_dir * -1.0;
        let pre_diffuse_color = light_color; // Base color comes from texture

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b =
                    NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, n1, n2, t1, t2);
            }

            // Unpack walker state
            let (
                x_start,
                x_end,
                z_left,
                q_left,
                u_left,
                v_left,
                nx_left,
                ny_left,
                nz_left,
                tx_left,
                ty_left,
                tz_left,
                tw_left,
            ) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.q,
                    edge_a.u,
                    edge_a.v,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                    edge_a.tx,
                    edge_a.ty,
                    edge_a.tz,
                    edge_a.tw,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.q,
                    edge_b.u,
                    edge_b.v,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                    edge_b.tx,
                    edge_b.ty,
                    edge_b.tz,
                    edge_b.tw,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_normal_mapped(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    NormalMapSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        tx: tx_left,
                        ty: ty_left,
                        tz: tz_left,
                        tw: tw_left,
                    },
                    &gradients,
                    texture,
                    normal_map,
                    neg_light_dir,
                    pre_diffuse_color,
                    ambient,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Draw a 3D line with Z-buffering.
///
/// Handles frustum clipping and perspective projection.
///
/// # Arguments
///
/// * `v0`, `v1` - Vertices defined as `(Position, W)`.
pub fn draw_line_3d(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    color: u32,
) {
    // Clip against frustum (returns None if fully culled)
    if let Some((v0_clipped, v1_clipped)) = clip_line_to_frustum(v0, v1, |v| *v) {
        let width = fb.width();
        let height = fb.height();
        let half_width = width as f32 * 0.5;
        let half_height = height as f32 * 0.5;

        // Project to screen
        let p0 = project_to_screen_optimized(v0_clipped.0, v0_clipped.1, half_width, half_height);
        let p1 = project_to_screen_optimized(v1_clipped.0, v1_clipped.1, half_width, half_height);

        // Bresenham's algorithm with Z interpolation
        // Standard integer-based line drawing
        let mut x0 = p0.x;
        let mut y0 = p0.y;
        let x1 = p1.x;
        let y1 = p1.y;

        // Z-interpolation (linear in screen space for simplicity/speed, though technically 1/z is linear)
        // For wireframes, linear Z is usually acceptable.
        let mut z = p0.z;
        let z_end = p1.z;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        // Calculate step size for Z interpolation
        // Total steps = max(|dx|, |dy|)
        let steps = dx.max(-dy);
        let dz = if steps > 0 {
            (z_end - z) / (steps as f32)
        } else {
            0.0
        };

        loop {
            // Check bounds (clipping should handle most cases, but guard against precision issues)
            if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                // Z-test
                // SAFETY: Bounds checked.
                unsafe {
                    let idx = (y0 as usize) * (width as usize) + (x0 as usize);
                    let z_buffer_val = zb.as_mut_slice().get_unchecked_mut(idx);
                    // Use standard depth test (less is closer for negative Z, wait.
                    // Project to screen produces z = v.z / w.
                    // If using standard OpenGL conventions, z is in [-1, 1].
                    // But rasterizer uses z < *depth_val.
                    // Let's assume standard behavior.
                    if z < *z_buffer_val {
                        *z_buffer_val = z;
                        fb.set_pixel_unchecked(x0 as usize, y0 as usize, color);
                    }
                }
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
            z += dz;
        }
    }
}

/// Fill a 3D triangle in wireframe mode.
///
/// Draws the three edges of the triangle as lines.
pub fn fill_triangle_wireframe(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    color: u32,
) {
    draw_line_3d(fb, zb, v0, v1, color);
    draw_line_3d(fb, zb, v1, v2, color);
    draw_line_3d(fb, zb, v2, v0, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Vec3;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn edge_walker_z_interpolation() {
        // Test that EdgeWalker correctly interpolates z
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // z should be 1.0
        assert!((walker.z - 1.0).abs() < 0.0001);

        // dz_dy should be (2.0 - 1.0) / 100.0 = 0.01
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        // Test that stepping accumulates z correctly
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);

        let initial_z = walker.z;
        let dz = walker.dz_dy;

        // Step 50 times
        walker.step_n(50);

        // After 50 steps, z should be initial + 50*dz
        let expected_z = initial_z + dz * 50.0;
        assert!((walker.z - expected_z).abs() < 0.0001);

        assert!(
            walker.z > 1.0 && walker.z < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            walker.z,
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        // Test that EdgeWalker handles degenerate case (zero height)
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // Should have zero gradients
        assert_eq!(walker.dx_dy, 0);
        assert!((walker.dz_dy - 0.0).abs() < f32::EPSILON);

        // z should still be correct
        assert!((walker.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn fill_triangle_3d_with_fixed_point_matches_reference() {
        // This is a regression test to ensure fixed-point conversion
        // does not change rendering output
        let width = 200;
        let height = 200;

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

        // Verify the triangle was rendered (at least some pixels changed)
        let rendered_pixels = fb.as_slice().iter().filter(|&&p| p != 0xFF00_0000).count();

        assert!(
            rendered_pixels > 100,
            "Expected at least 100 pixels rendered, got {rendered_pixels}",
        );

        // Verify center pixel is red (triangle is centered)
        let center_pixel = fb.get_pixel((width / 2) as i32, (height / 2) as i32);
        assert_eq!(
            center_pixel,
            Some(color),
            "Center pixel should be red (triangle color)"
        );
    }

    #[test]
    fn draw_scanline_flat_interpolation() {
        // Test that draw_scanline_flat correctly interpolates z
        let width = 100;
        let height = 1;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Draw a scanline with float z
        let z_start = 5.0;
        let dz_dx = 0.01; // Slight gradient
        let color = 0xFFFF_0000;

        draw_scanline_flat(&mut fb, &mut zb, 0, 0, 99, z_start, dz_dx, color);

        // Verify all pixels were drawn
        for x in 0..width {
            assert_eq!(
                fb.get_pixel(x as i32, 0),
                Some(color),
                "Pixel at x={x} should be colored",
            );
        }

        // Verify zbuffer was updated correctly
        let zb_slice = zb.as_slice();
        assert!(
            (zb_slice[0] - 5.0).abs() < 0.0001,
            "First pixel: got {}, expected 5.0",
            zb_slice[0]
        );
        assert!(
            zb_slice[50] > 5.0 && zb_slice[50] < 6.0,
            "Middle pixel: got {}, expected between 5.0 and 6.0",
            zb_slice[50]
        );
        assert!(
            zb_slice[99] > 5.0 && zb_slice[99] < 7.0,
            "Last pixel: got {}, expected between 5.0 and 7.0",
            zb_slice[99]
        );
    }

    #[test]
    fn test_is_backface_overflow() {
        // Construct points that maximize the coordinate differences
        // p0 at (MIN, MIN)
        let p0 = ScreenPoint {
            x: i32::MIN,
            y: i32::MIN,
            z: 0.0,
            inv_w: 1.0,
        };
        // p1 at (MAX, MIN) -> ux = MAX - MIN approx 4e9
        let p1 = ScreenPoint {
            x: i32::MAX,
            y: i32::MIN,
            z: 0.0,
            inv_w: 1.0,
        };
        // p2 at (MIN, MAX) -> vy = MAX - MIN approx 4e9
        let p2 = ScreenPoint {
            x: i32::MIN,
            y: i32::MAX,
            z: 0.0,
            inv_w: 1.0,
        };

        // ux * vy approx 1.6e19, which exceeds i64::MAX (9e18)
        // This should not panic
        let result = is_backface(p0, p1, p2);

        assert!(result);
    }
}
