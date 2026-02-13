use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

// Fixed point scale factor (16.16)
pub(crate) const FIXED_SCALE: f32 = 65536.0;

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
pub fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
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
pub fn is_backface(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
    let ux = i64::from(p1.x) - i64::from(p0.x);
    let uy = i64::from(p1.y) - i64::from(p0.y);
    let vx = i64::from(p2.x) - i64::from(p0.x);
    let vy = i64::from(p2.y) - i64::from(p0.y);
    // Use i128 to prevent overflow during cross product calculation for large coordinates
    let nz = i128::from(ux) * i128::from(vy) - i128::from(uy) * i128::from(vx);
    nz >= 0
}

pub struct EdgeWalker {
    pub(crate) x: i64,
    dx_dy: i64,
    pub(crate) z: f32,
    dz_dy: f32,
}

impl EdgeWalker {
    pub fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
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

    pub fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
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
pub(crate) fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
pub(crate) const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
pub(crate) fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
}

pub const RECIPROCAL_TABLE: [f32; 17] = [
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ScreenPoint;

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
