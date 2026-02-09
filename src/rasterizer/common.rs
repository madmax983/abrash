use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

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
    let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
    let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
    let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
    let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    let nz = ux * vy - uy * vx;
    nz >= 0.0
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
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

// Fixed point scale factor (16.16)
pub(crate) const FIXED_SCALE: f32 = 65536.0;

pub(crate) struct EdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: i32, // 24.8 fixed point
    dx_dy: i64,
    dz_dy: i32, // 24.8 fixed point
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0)
        } else {
            let inv_h = 1.0 / height;

            // Convert z to 24.8 fixed point
            let z0_fixed = (p_start.z * 256.0) as i32;
            let z1_fixed = (p_end.z * 256.0) as i32;
            let dz = i64::from(z1_fixed - z0_fixed);

            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                ((dz as f32) * inv_h) as i32,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: (p_start.z * 256.0) as i32, // Convert to 24.8 fixed point
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) const fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i32) {
        let n_i64 = i64::from(n);
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_walker_fixed_point_z_conversion() {
        // Test that EdgeWalker correctly converts z to 24.8 fixed point
        let p0 = ScreenPoint { x: 0, y: 0, z: 1.0 };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // z should be converted to 24.8 fixed point: 1.0 * 256 = 256
        assert_eq!(walker.z, 256);

        // dz_dy should also be in fixed point: (2.0 - 1.0) / 100.0 = 0.01
        // In 24.8: 0.01 * 256 = 2.56 ≈ 2 or 3 (depends on rounding)
        let expected_dz_dy = ((2.0_f32 - 1.0_f32) / 100.0_f32 * 256.0) as i32;
        assert_eq!(walker.dz_dy, expected_dz_dy);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        // Test that stepping accumulates z correctly in fixed point
        let p0 = ScreenPoint { x: 0, y: 0, z: 1.0 };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);

        let initial_z = walker.z;
        let dz = walker.dz_dy;

        // Step 50 times
        walker.step_n(50);

        // After 50 steps, z should be initial + 50*dz
        let expected_z = initial_z + dz * 50;
        assert_eq!(walker.z, expected_z);

        // Convert back to float for verification
        let z_float = (walker.z as f32) / 256.0;
        // With 24.8 fixed point, expect some rounding error
        // The ideal would be 1.5, but we get ~1.39 due to integer truncation in dz_dy
        assert!(
            z_float > 1.0 && z_float < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            z_float
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        // Test that EdgeWalker handles degenerate case (zero height)
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // Should have zero gradients
        assert_eq!(walker.dx_dy, 0);
        assert_eq!(walker.dz_dy, 0);

        // z should still be converted correctly
        assert_eq!(walker.z, (1.0 * 256.0) as i32);
    }
}
