//! Retro pseudo-3D tunnel effect.
//!
//! Simulates moving through an infinite 3D tunnel using polar coordinate math.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a 3D tunnel effect to the framebuffer.
///
/// This filter converts Cartesian screen coordinates (x, y) into polar coordinates
/// (angle, distance) to map a procedural texture (like XOR texture) onto the inside of a cylinder,
/// creating the illusion of moving forward through an endless tunnel.
///
/// * `fb`: The Framebuffer to modify.
/// * `time`: A time variable used to animate forward motion and rotation.
pub fn apply_tunnel(fb: &mut Framebuffer, time: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let tex_size = 256.0;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let dy = y as f32 - cy;

        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let dx = x as f32 - cx;

            // Calculate polar coordinates
            let distance = (dx * dx + dy * dy).sqrt();
            if distance == 0.0 {
                *pixel = 0xFF_00_00_00;
                continue;
            }

            let angle = dy.atan2(dx);

            // Map to UV space
            // U: Wrap around the tunnel based on angle, add time for rotation
            // V: Move into the tunnel based on inverse distance, add time for forward motion
            let u = (angle / std::f32::consts::PI) * 0.5 + time * 0.2; // 0.0 to 1.0 (wrapped)
            let v = (cx.max(cy) / distance) + time * 1.5;

            // Map UV to an XOR procedural texture
            let tex_u = ((u * tex_size) as i32) & 255;
            let tex_v = ((v * tex_size) as i32) & 255;

            let xor_val = tex_u ^ tex_v;

            // Apply simple distance fog
            let fog_factor = (distance / (cx.max(cy))).clamp(0.0, 1.0);

            let r = (xor_val as f32 * fog_factor) as u32;
            let g = ((xor_val as f32 * 0.5) * fog_factor) as u32;
            let b = ((xor_val as f32 * 2.0).min(255.0) * fog_factor) as u32;

            *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_tunnel_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        apply_tunnel(&mut fb, 0.0);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_apply_tunnel_standard() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_00_00_00);
        apply_tunnel(&mut fb, 0.0);

        // Verify that the framebuffer has been altered and is no longer black.
        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Framebuffer should not be entirely black after tunnel effect"
        );
    }
}
