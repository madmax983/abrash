//! VHS Tracking Glitch Filter
//!
//! A retro post-processing effect that simulates the tracking distortion,
//! chromatic aberration, and noise characteristic of degraded analog video tape (VHS).

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static VHS_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the VHS Tracking effect.
#[derive(Debug, Clone, Copy)]
pub struct VhsConfig {
    /// How much the tracking band disrupts the image (0.0 to 1.0).
    pub intensity: f32,
    /// A continuously increasing time value used to animate the noise and the tracking band.
    pub time: f32,
    /// The normalized vertical position of the tracking band (0.0 = top, 1.0 = bottom).
    /// Typically, you animate this so the band rolls down or up the screen.
    pub tracking_position: f32,
    /// The normalized thickness of the tracking band (e.g., 0.1 for 10% of the screen height).
    pub tracking_thickness: f32,
    /// The intensity of the global color noise added to the image (0.0 to 1.0).
    pub noise_intensity: f32,
}

impl Default for VhsConfig {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            time: 0.0,
            tracking_position: 0.5,
            tracking_thickness: 0.1,
            noise_intensity: 0.2,
        }
    }
}

/// Applies a VHS tracking glitch effect to the framebuffer.
///
/// This effect combines:
/// 1. A moving horizontal "tracking band" that displaces and shears pixels.
/// 2. Localized chromatic aberration (RGB channel separation) within the band.
/// 3. Additive luminance/chroma noise simulating magnetic tape degradation.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_vhs(fb: &mut Framebuffer, config: &VhsConfig) {
    if config.intensity <= 0.0 && config.noise_intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // We must read from the original pixels and write to a new buffer
    // because pixels are displaced horizontally and channels are shifted.
    VHS_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        // Calculate tracking band bounds in pixel coordinates
        let tracking_center = (config.tracking_position * height as f32) as i32;
        let tracking_half_thickness = (config.tracking_thickness * height as f32 * 0.5) as i32;

        let tracking_top = tracking_center - tracking_half_thickness;
        let tracking_bottom = tracking_center + tracking_half_thickness;

        // Scale max shifts by intensity
        let max_horizontal_shift = (width as f32 * 0.05 * config.intensity) as i32;
        let max_chroma_shift = (width as f32 * 0.02 * config.intensity) as i32;

        // Base seed for noise
        let global_seed = (config.time * 1000.0) as u32 ^ 0x1337_C455;

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let mut prng_state = global_seed.wrapping_add((y as u32).wrapping_mul(7919));
            if prng_state == 0 {
                prng_state = 1;
            }
            let mut prng = XorShift32::new(prng_state);

            let is_in_tracking_band = (y as i32) >= tracking_top && (y as i32) <= tracking_bottom;

            // Global noise calculation for this row
            // We'll generate a few random values per row to add horizontal streaks
            let row_noise = (prng.next_u32() % 255) as i32;
            let streak_noise = (prng.next_u32() % 100) as f32 / 100.0;

            let (horizontal_shift, r_shift, b_shift) = if is_in_tracking_band {
                // Determine how deep into the band we are (0.0 at edges, 1.0 at center)
                let dist_to_center = (y as i32 - tracking_center).abs() as f32;
                let band_intensity =
                    1.0 - (dist_to_center / tracking_half_thickness as f32).max(0.0).min(1.0);

                // Add a sine wave oscillation to the shift for a "wobble" effect
                let wobble = (config.time * 10.0 + y as f32 * 0.1).sin();

                let mut h_shift = (wobble * max_horizontal_shift as f32 * band_intensity) as i32;

                // Add jagged random jitter inside the band
                if streak_noise < 0.3 {
                    h_shift += (prng.next_u32() % (max_horizontal_shift.max(1) as u32 * 2 + 1))
                        as i32
                        - max_horizontal_shift;
                }

                let r_s = (max_chroma_shift as f32 * band_intensity) as i32;
                let b_s = -(max_chroma_shift as f32 * band_intensity) as i32;

                (h_shift, r_s, b_s)
            } else {
                // Very slight random jitter outside the band if intensity is high
                let h_shift = if config.intensity > 0.8 && streak_noise < 0.05 {
                    (prng.next_u32() % 5) as i32 - 2
                } else {
                    0
                };
                (h_shift, 0, 0)
            };

            for (x, pixel) in row.iter_mut().enumerate() {
                let base_x = x as i32 - horizontal_shift;

                // Sample channels with chromatic aberration shift
                let sample_r =
                    get_channel_safe(src_pixels, width, height, base_x - r_shift, y as i32, 16);
                let sample_g = get_channel_safe(src_pixels, width, height, base_x, y as i32, 8);
                let sample_b =
                    get_channel_safe(src_pixels, width, height, base_x - b_shift, y as i32, 0);

                // Add noise
                let mut final_r = sample_r as i32;
                let mut final_g = sample_g as i32;
                let mut final_b = sample_b as i32;

                if config.noise_intensity > 0.0 {
                    // Fast pseudo-random value per pixel based on row noise
                    let pixel_noise = ((x as u32)
                        .wrapping_mul(1_664_525)
                        .wrapping_add(row_noise as u32)
                        % 255) as i32;
                    let noise_val = ((pixel_noise - 128) as f32 * config.noise_intensity) as i32;

                    final_r = (final_r + noise_val).max(0).min(255);
                    final_g = (final_g + noise_val).max(0).min(255);
                    final_b = (final_b + noise_val).max(0).min(255);
                }

                // Add static "snow" specks
                if is_in_tracking_band && (x as u32 ^ prng.next_u32()) % 100 < 5 {
                    final_r = 200;
                    final_g = 200;
                    final_b = 200;
                }

                *pixel = 0xFF00_0000
                    | ((final_r as u32) << 16)
                    | ((final_g as u32) << 8)
                    | (final_b as u32);
            }
        });
    });
}

/// Helper function to safely sample a specific color channel from the source buffer,
/// clamping to the edges of the row if the shift goes out of bounds.
#[inline(always)]
fn get_channel_safe(src: &[u32], width: usize, _height: usize, x: i32, y: i32, shift: u8) -> u32 {
    let clamped_x = x.max(0).min(width as i32 - 1) as usize;
    let idx = y as usize * width + clamped_x;
    (src[idx] >> shift) & 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_vhs_changes_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Draw a vertical white line down the middle on a black background
        fb.clear(0xFF00_0000);
        for y in 0..height {
            fb.set_pixel(50, y as i32, 0xFFFF_FFFF);
        }

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        let config = VhsConfig {
            intensity: 1.0,
            time: 42.0,
            tracking_position: 0.5,
            tracking_thickness: 0.2,
            noise_intensity: 0.5,
        };

        // Apply VHS effect
        apply_vhs(&mut fb, &config);

        // Verify the buffer was modified
        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(different, "VHS filter did not modify the framebuffer");
    }

    #[test]
    fn test_apply_vhs_zero_intensity() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF12_3456);

        let config = VhsConfig {
            intensity: 0.0,
            time: 1.0,
            tracking_position: 0.5,
            tracking_thickness: 0.1,
            noise_intensity: 0.0,
        };

        apply_vhs(&mut fb, &config);

        // Buffer should be untouched
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF12_3456);
        }
    }
}
