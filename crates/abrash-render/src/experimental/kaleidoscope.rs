//! Kaleidoscope Post-Processing Filter
//!
//! Applies a symmetric, radial mirror effect to the framebuffer,
//! simulating a classic kaleidoscope by mapping pixels to polar
//! coordinates, applying modulo to the angle, and mapping back.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static KALEIDOSCOPE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a kaleidoscope effect to the framebuffer.
///
/// Divides the screen into `segments` radially, and mirrors the content
/// of the first segment across the others.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `segments` - The number of mirror segments (e.g. 6). Must be > 1 to have an effect.
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
#[inline(always)]
#[must_use]
pub fn fast_atan2(y: f32, x: f32) -> f32 {
    let abs_y = y.abs();
    let abs_x = x.abs();

    if abs_x == 0.0 && abs_y == 0.0 {
        return 0.0;
    }

    let mut theta = if abs_x > abs_y {
        let ratio = (abs_x - abs_y) / (abs_x + abs_y);
        0.25 * std::f32::consts::PI * (1.0 - ratio)
    } else {
        let ratio = (abs_y - abs_x) / (abs_x + abs_y);
        0.25 * std::f32::consts::PI * (1.0 + ratio)
    };

    if x < 0.0 {
        theta = std::f32::consts::PI - theta;
    }
    if y < 0.0 {
        theta = -theta;
    }

    if theta < 0.0 {
        theta += std::f32::consts::TAU;
    }

    theta
}

pub fn apply_kaleidoscope(fb: &mut Framebuffer, segments: usize) {
    if segments <= 1 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // The angle of a single segment
    let segment_angle = std::f32::consts::TAU / segments as f32;

    // We must clone the buffer to read from the original state while writing to the new state.
    // This avoids artifacts from reading already-modified pixels.
    // ⚡ Bolt: Use a thread-local buffer to eliminate per-frame dynamic heap allocations.
    KALEIDOSCOPE_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            dest_pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    let dy = y as f32 - cy;

                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx;

                        // Convert to polar coordinates
                        #[allow(clippy::imprecise_flops)]
                        let r = (dx * dx + dy * dy).sqrt();

                        // ⚡ Bolt: Fast mathematical approximation for atan2 to reduce overhead
                        let mut theta = fast_atan2(dy, dx);

                        // Apply modulo to find the angle within the first segment
                        let original_theta = theta;
                        theta %= segment_angle;

                        // Mirror every other segment for true kaleidoscope symmetry
                        let segment_index = (original_theta / segment_angle) as usize;
                        if segment_index % 2 == 1 {
                            // Mirror it
                            theta = segment_angle - theta;
                        }

                        // Convert back to Cartesian
                        // Using fast float-to-int cast saves overhead when exact rounding isn't required
                        let sample_x = (cx + r * theta.cos()) as i32;
                        let sample_y = (cy + r * theta.sin()) as i32;

                        // Clamp coordinates to stay within bounds
                        let clamped_x = sample_x.clamp(0, width as i32 - 1) as usize;
                        let clamped_y = sample_y.clamp(0, height as i32 - 1) as usize;

                        *pixel = src_fb[clamped_y * width + clamped_x];
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let row_start = y * width;
                let dy = y as f32 - cy;

                for x in 0..width {
                    let dx = x as f32 - cx;

                    // Convert to polar coordinates
                    #[allow(clippy::imprecise_flops)]
                    let r = (dx * dx + dy * dy).sqrt();

                    // ⚡ Bolt: Fast mathematical approximation for atan2 to reduce overhead
                    let mut theta = fast_atan2(dy, dx);

                    // Apply modulo to find the angle within the first segment
                    let original_theta = theta;
                    theta %= segment_angle;

                    // Mirror every other segment for true kaleidoscope symmetry
                    let segment_index = (original_theta / segment_angle) as usize;
                    if segment_index % 2 == 1 {
                        // Mirror it
                        theta = segment_angle - theta;
                    }

                    // Convert back to Cartesian
                    let sample_x = (cx + r * theta.cos()) as i32;
                    let sample_y = (cy + r * theta.sin()) as i32;

                    // Clamp coordinates to stay within bounds
                    let clamped_x = sample_x.clamp(0, width as i32 - 1) as usize;
                    let clamped_y = sample_y.clamp(0, height as i32 - 1) as usize;

                    dest_pixels[row_start + x] = src_fb[clamped_y * width + clamped_x];
                }
            }
        }
    });
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_fast_atan2_cardinal_directions() {
        let epsilon = 0.001;
        // fast_atan2 returns [0, TAU] where 0 is the positive X axis.
        // std atan2 returns [-PI, PI], so we adjust std to [0, TAU].
        let points = [
            (1.0, 0.0, std::f32::consts::FRAC_PI_2), // +Y (up)
            (0.0, 1.0, 0.0),                         // +X (right)
            (
                -1.0,
                0.0,
                std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            ), // -Y (down)
            (0.0, -1.0, std::f32::consts::PI),       // -X (left)
            (1.0, 1.0, std::f32::consts::FRAC_PI_4), // Top-Right
            (
                -1.0,
                1.0,
                std::f32::consts::PI + std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4,
            ), // Bottom-Right
            (
                1.0,
                -1.0,
                std::f32::consts::PI - std::f32::consts::FRAC_PI_4,
            ), // Top-Left
            (
                -1.0,
                -1.0,
                std::f32::consts::PI + std::f32::consts::FRAC_PI_4,
            ), // Bottom-Left
        ];

        for (y, x, expected) in points.iter() {
            let mut std_atan2 = (*y as f32).atan2(*x as f32);
            if std_atan2 < 0.0 {
                std_atan2 += std::f32::consts::TAU;
            }
            let fast = fast_atan2(*y, *x);

            // Allow for a max error of about 4 degrees (0.07 rads) for the approximation
            assert!(
                (fast - expected).abs() < 0.08,
                "fast_atan2({}, {}) = {} vs expected std {}",
                y,
                x,
                fast,
                expected
            );
        }
    }

    #[test]
    fn test_kaleidoscope_segments_zero_one() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFFFFFF);
        // Original logic: 1 segment or 0 should do nothing
        apply_kaleidoscope(&mut fb, 0);
        assert_eq!(fb.get_pixel(50, 50), Some(0xFFFFFFFF));
        apply_kaleidoscope(&mut fb, 1);
        assert_eq!(fb.get_pixel(50, 50), Some(0xFFFFFFFF));
    }

    #[test]
    fn test_kaleidoscope_symmetry() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF000000); // Black
        // Draw something non-symmetric in the base segment [0, 60 degrees], which is roughly x > 50, y > 50
        for y in 50..100 {
            for x in 50..100 {
                fb.set_pixel(x, y, 0xFFFFFFFF); // White bottom-right
            }
        }

        apply_kaleidoscope(&mut fb, 6);

        // After kaleidoscope (6 segments), the original bottom-right quadrant should be mirrored
        // and rotated to fill the entire screen symmetrically.
        // Let's check a point in the top-left quadrant (e.g., x=25, y=25), which should
        // have sampled white.

        let mut found_white_elsewhere = false;
        for y in 0..50 {
            for x in 0..50 {
                if fb.get_pixel(x, y) == Some(0xFFFFFFFF) {
                    found_white_elsewhere = true;
                    break;
                }
            }
        }

        assert!(
            found_white_elsewhere,
            "Kaleidoscope should have mirrored content to the top left side"
        );
    }
}
