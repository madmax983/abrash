use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

/// Fixed point scale factor (16.16)
pub const FIXED_SCALE: f32 = 65536.0;

/// Helper to ensure buffer dimensions match
#[inline]
pub(crate) fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
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
pub(crate) fn prepare_scanline<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    mut z: f32,
    dz_dx: f32,
) -> Option<(&'a mut [u32], &'a mut [f32], f32)> {
    if y < 0 || y >= fb.height() as i32 {
        return None;
    }

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
    // Use i64 for cross product.
    // i64 is sufficient as long as viewport width < 3e9, which is enforced by Framebuffer::new.
    let nz = ux * vy - uy * vx;
    nz >= 0
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
pub fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
pub const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
pub fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
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

/// Helper to calculate gradients for triangle rasterization.
///
/// This struct pre-calculates the screen-space derivatives (d/dx, d/dy)
/// for barycentric coordinates, which are then used to interpolate vertex attributes.
pub struct GradientContext {
    inv_nz: f32,
    ux: f32,
    uy: f32,
    vx: f32,
    vy: f32,
}

impl GradientContext {
    /// Creates a new GradientContext from three screen-space points.
    ///
    /// Returns the context and a boolean indicating if the winding order is front-facing (positive area).
    pub fn new(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        (
            Self {
                inv_nz,
                ux,
                uy,
                vx,
                vy,
            },
            nz > 0.0,
        )
    }

    /// Calculate the X derivative (d/dx) for an attribute.
    ///
    /// `val0`, `val1`, `val2` are the attribute values at p0, p1, p2 respectively.
    #[inline(always)]
    pub fn calculate_dx(&self, val0: f32, val1: f32, val2: f32) -> f32 {
        let u_val = val1 - val0;
        let v_val = val2 - val0;
        let nx_val = self.uy * v_val - u_val * self.vy;
        nx_val * self.inv_nz
    }

    /// Calculate the Y derivative (d/dy) for an attribute.
    #[inline(always)]
    pub fn calculate_dy(&self, val0: f32, val1: f32, val2: f32) -> f32 {
        let u_val = val1 - val0;
        let v_val = val2 - val0;
        let ny_val = u_val * self.vx - self.ux * v_val;
        ny_val * self.inv_nz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ScreenPoint;

    #[test]
    fn edge_walker_z_interpolation() {
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

        assert!((walker.z - 1.0).abs() < 0.0001);
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
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

        walker.step_n(50);
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
        assert_eq!(walker.dx_dy, 0);
        assert!((walker.dz_dy - 0.0).abs() < f32::EPSILON);
        assert!((walker.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_is_backface_overflow_safe() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 2_000_000_000,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p2 = ScreenPoint {
            x: 0,
            y: 2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };

        let result = is_backface(p0, p1, p2);
        assert!(result);
    }

    #[test]
    fn test_gradient_context() {
        // Simple right triangle: (0,0), (10,0), (0,10)
        // Z values: 0, 10, 10
        // Expected dz/dx = 1.0, dz/dy = 1.0
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 10,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p2 = ScreenPoint {
            x: 0,
            y: 10,
            z: 0.0,
            inv_w: 1.0,
        };

        let (ctx, _) = GradientContext::new(p0, p1, p2);

        // Test dz/dx
        // z0=0, z1=10, z2=10
        let dz_dx = ctx.calculate_dx(0.0, 10.0, 10.0);
        let dz_dy = ctx.calculate_dy(0.0, 10.0, 10.0);

        // ux = 10, uy = 0
        // vx = 0, vy = 10
        // nz = 10*10 - 0*0 = 100
        // inv_nz = -0.01

        // dz_dx calculation:
        // u_val = 10, v_val = 10
        // nx = uy * v_val - u_val * vy = 0*10 - 10*10 = -100
        // dx = -100 * -0.01 = 1.0. Correct.

        assert!((dz_dx - 1.0).abs() < 0.0001, "dz_dx was {}", dz_dx);

        // dz_dy calculation:
        // ny = u_val * vx - ux * v_val = 10*0 - 10*10 = -100
        // dy = -100 * -0.01 = 1.0. Correct.

        assert!((dz_dy - 1.0).abs() < 0.0001, "dz_dy was {}", dz_dy);
    }
}
