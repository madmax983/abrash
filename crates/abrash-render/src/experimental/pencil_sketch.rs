//! Pencil Sketch Post-Processing Filter
//!
//! A non-photorealistic post-processing effect that converts the framebuffer
//! into a stylized pencil sketch. It works by converting the image to grayscale,
//! inverting it, applying a box blur, and then blending the blurred inverted
//! image with the original grayscale image using a Color Dodge blend mode.
//! This technique naturally extracts edges and shading to simulate pencil strokes.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static SKETCH_BUFFER_1: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SKETCH_BUFFER_2: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Pencil Sketch effect.
#[derive(Debug, Clone, Copy)]
pub struct PencilSketchConfig {
    /// Intensity of the sketch effect blending (0.0 to 1.0).
    pub intensity: f32,
    /// Radius of the blur applied to the inverted image. Higher values result in thicker lines and softer shading.
    pub blur_radius: i32,
}

impl Default for PencilSketchConfig {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            blur_radius: 5,
        }
    }
}

/// Applies a Pencil Sketch effect to the framebuffer.
///
/// # Performance
///
/// Uses thread-local buffers for intermediate steps (grayscale inversion and blur passes)
/// to avoid dynamic allocations per frame, allowing real-time performance.
pub fn apply_pencil_sketch(fb: &mut Framebuffer, config: &PencilSketchConfig) {
    if config.intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let size = width * height;
    let radius = config.blur_radius.max(1) as usize;

    SKETCH_BUFFER_1.with(|buf1| {
        SKETCH_BUFFER_2.with(|buf2| {
            let mut luma_inv_vec = buf1.borrow_mut();
            if luma_inv_vec.len() < size {
                luma_inv_vec.resize(size, 0);
            }
            let luma_inv = &mut luma_inv_vec[..size];

            let mut blurred_vec = buf2.borrow_mut();
            if blurred_vec.len() < size {
                blurred_vec.resize(size, 0);
            }
            let blurred = &mut blurred_vec[..size];

            let pixels = fb.as_mut_slice();

            // 1. Convert to grayscale and invert, store in luma_inv
            // At the same time, store original luma in the alpha channel of pixels
            // so we don't have to recalculate it later for the blend.
            #[cfg(feature = "parallel")]
            let iter = pixels.par_iter_mut().zip(luma_inv.par_iter_mut());
            #[cfg(not(feature = "parallel"))]
            let iter = pixels.iter_mut().zip(luma_inv.iter_mut());

            iter.for_each(|(pixel, inv_dest)| {
                let luma = pixel_luminance(*pixel);
                *inv_dest = 255 - luma;
                // Pack original luma into alpha (we'll ignore the old alpha anyway)
                *pixel = (*pixel & 0x00FF_FFFF) | (u32::from(luma) << 24);
            });

                        // 2. Separable Box Blur
            // Horizontal pass: luma_inv -> blurred
            #[cfg(feature = "parallel")]
            let h_iter = blurred.par_chunks_exact_mut(width).enumerate();
            #[cfg(not(feature = "parallel"))]
            let h_iter = blurred.chunks_exact_mut(width).enumerate();

            h_iter.for_each(|(y, row_dest)| {
                let row_start = y * width;
                let src_row = &luma_inv[row_start..row_start + width];

                let mut sum = 0;
                let window_size = (radius * 2 + 1) as u32;

                // Prime the window (centered)
                for x in 0..=radius {
                    sum += u32::from(src_row[x.min(width - 1)]);
                }
                for _ in 0..radius {
                    sum += u32::from(src_row[0]); // Left clamp
                }

                for x in 0..width {
                    row_dest[x] = (sum / window_size) as u8;

                    let trailing_x = x.saturating_sub(radius);
                    let leading_x = (x + radius + 1).min(width - 1);

                    sum -= u32::from(src_row[trailing_x]);
                    sum += u32::from(src_row[leading_x]);
                }
            });

            // Transpose blurred into luma_inv for cache-friendly vertical blur
            for y in 0..height {
                for x in 0..width {
                    luma_inv[x * height + y] = blurred[y * width + x];
                }
            }

            // Vertical pass (now horizontal on the transposed buffer): luma_inv -> blurred
            #[cfg(feature = "parallel")]
            let v_iter = blurred.par_chunks_exact_mut(height).enumerate();
            #[cfg(not(feature = "parallel"))]
            let v_iter = blurred.chunks_exact_mut(height).enumerate();

            v_iter.for_each(|(x, col_dest)| {
                let col_start = x * height;
                let src_col = &luma_inv[col_start..col_start + height];

                let mut sum = 0;
                let window_size = (radius * 2 + 1) as u32;

                // Prime the window (centered)
                for y in 0..=radius {
                    sum += u32::from(src_col[y.min(height - 1)]);
                }
                for _ in 0..radius {
                    sum += u32::from(src_col[0]); // Top clamp
                }

                for y in 0..height {
                    col_dest[y] = (sum / window_size) as u8;

                    let trailing_y = y.saturating_sub(radius);
                    let leading_y = (y + radius + 1).min(height - 1);

                    sum -= u32::from(src_col[trailing_y]);
                    sum += u32::from(src_col[leading_y]);
                }
            });

            // Transpose blurred back into luma_inv
            for x in 0..width {
                for y in 0..height {
                    luma_inv[y * width + x] = blurred[x * height + y];
                }
            }

            // 3. Color Dodge Blend
            // Base = original luma (stored in alpha channel of pixels)
            // Blend = blurred inverted luma (stored in luma_inv)
            // Result = Base / (1.0 - Blend) => in integer: (Base * 255) / (255 - Blend)

            #[cfg(feature = "parallel")]
            let blend_iter = pixels.par_iter_mut().zip(luma_inv.par_iter());
            #[cfg(not(feature = "parallel"))]
            let blend_iter = pixels.iter_mut().zip(luma_inv.iter());

            let intensity_clamped = config.intensity.clamp(0.0, 1.0);

            blend_iter.for_each(|(pixel, blend_val)| {
                let base = (*pixel >> 24) as u32; // Extract original luma
                let blend = u32::from(*blend_val);

                let dodge = if blend == 255 {
                    255
                } else {
                    ((base * 255) / (255 - blend)).min(255)
                };

                // Blend with original base based on intensity
                let final_luma = (base as f32 * (1.0 - intensity_clamped)
                    + dodge as f32 * intensity_clamped) as u32;

                // Reconstruct pixel (grayscale, alpha = 255)
                *pixel = 0xFF00_0000 | (final_luma << 16) | (final_luma << 8) | final_luma;
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pencil_sketch_basic() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Fill with some pattern
        for y in 0..4 {
            for x in 0..4 {
                fb.set_pixel(x, y, 0xFF00_0000 | (x as u32 * 40 + y as u32 * 40));
            }
        }

        let config = PencilSketchConfig {
            intensity: 1.0,
            blur_radius: 1,
        };

        apply_pencil_sketch(&mut fb, &config);

        // Just verify it doesn't panic and modifies the buffer
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p >> 24, 0xFF); // Alpha should be reset to FF
    }

    #[test]
    fn test_pencil_sketch_zero_intensity() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF12_3456);

        let config = PencilSketchConfig {
            intensity: 0.0,
            blur_radius: 1,
        };

        apply_pencil_sketch(&mut fb, &config);

        // Should be unmodified
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF12_3456));
    }
}
