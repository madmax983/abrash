//! Video Feedback Post-Processing Effect
//!
//! A retro post-processing effect that simulates video feedback (pointing a camera at its own monitor).
//! It maintains a history buffer and blends it back onto the current frame with a transformation.

use crate::framebuffer::Framebuffer;
use abrash_core::color::Color;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static HISTORY_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Video Feedback effect.
#[derive(Debug, Clone, Copy)]
pub struct VideoFeedbackConfig {
    /// Blend factor for the history buffer (0.0 to 1.0). 0.0 is no feedback, 1.0 is full feedback.
    pub feedback_amount: f32,
    /// Rotation applied to the history buffer before blending (in radians).
    pub rotation: f32,
    /// Scale applied to the history buffer before blending. > 1.0 zooms in, < 1.0 zooms out.
    pub scale: f32,
    /// Horizontal offset applied to the history buffer (normalized, 0.0 to 1.0).
    pub offset_x: f32,
    /// Vertical offset applied to the history buffer (normalized, 0.0 to 1.0).
    pub offset_y: f32,
}

impl Default for VideoFeedbackConfig {
    fn default() -> Self {
        Self {
            feedback_amount: 0.9,
            rotation: 0.01,
            scale: 0.99,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Applies a video feedback effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_video_feedback(fb: &mut Framebuffer, config: &VideoFeedbackConfig) {
    if config.feedback_amount <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    HISTORY_BUFFER.with(|buf| {
        let mut history = buf.borrow_mut();
        let size = width * height;
        if history.len() != size {
            history.resize(size, 0xFF00_0000); // Initialize with black
        }

        let fb_pixels = fb.as_mut_slice();

        // Transformation parameters (inverse mapping)
        // To map the history *forward* by applying rotation, scale, and translation,
        // we must iterate over the current pixels and apply the *inverse* transformation
        // to find the corresponding pixel in the history buffer.

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        // Inverse scale
        let inv_scale = 1.0 / config.scale;

        // Inverse rotation (negative angle)
        let (sin_r, cos_r) = (-config.rotation).sin_cos();

        // Inverse translation (subtract offset)
        let tx = -config.offset_x * width as f32;
        let ty = -config.offset_y * height as f32;

        let alpha = config.feedback_amount.clamp(0.0, 1.0);

        let hist_slice = history.as_slice();
        #[cfg(feature = "parallel")]
        let row_iter = fb_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = fb_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let y_f = y as f32;

            for (x, pixel) in row.iter_mut().enumerate() {
                let x_f = x as f32;

                // 1. Translate center to origin
                let dx = x_f - cx;
                let dy = y_f - cy;

                // 2. Apply inverse translation
                let dx = dx + tx;
                let dy = dy + ty;

                // 3. Apply inverse rotation
                let rx = dx * cos_r - dy * sin_r;
                let ry = dx * sin_r + dy * cos_r;

                // 4. Apply inverse scale
                let sx = rx * inv_scale;
                let sy = ry * inv_scale;

                // 5. Translate back from origin to center
                let src_x = sx + cx;
                let src_y = sy + cy;

                // Bilinear filtering or nearest neighbor? Let's use nearest neighbor for retro feel and speed
                let hist_pixel = if src_x >= 0.0 && src_x < width as f32 && src_y >= 0.0 && src_y < height as f32 {
                    let ix = src_x as usize;
                    let iy = src_y as usize;
                    hist_slice[iy * width + ix]
                } else {
                    0xFF00_0000 // default to black if out of bounds
                };

                // Blend history with current pixel
                let c_cur = Color::from_argb_u32(*pixel);
                let c_hist = Color::from_argb_u32(hist_pixel);

                // Let's do max for a very clean retro feedback loop (like light trails).
                let cur_r = (c_cur.r * 255.0).max(c_hist.r * 255.0 * alpha) as u8;
                let cur_g = (c_cur.g * 255.0).max(c_hist.g * 255.0 * alpha) as u8;
                let cur_b = (c_cur.b * 255.0).max(c_hist.b * 255.0 * alpha) as u8;

                let blended =
                    0xFF00_0000 | (u32::from(cur_r) << 16) | (u32::from(cur_g) << 8) | u32::from(cur_b);

                *pixel = blended;
            }
        });

        // Save current frame as new history for the next frame
        history.copy_from_slice(fb_pixels);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_video_feedback() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF00_0000); // Black background

        // Draw a white pixel in the center
        fb.set_pixel(5, 5, 0xFFFF_FFFF);

        let config = VideoFeedbackConfig {
            feedback_amount: 0.5, // 50% blend
            rotation: 0.0,
            scale: 1.0,
            offset_x: 0.1, // shift right by 1 pixel (10 * 0.1 = 1)
            offset_y: 0.0,
        };

        // Clear history for testing
        HISTORY_BUFFER.with(|buf| buf.borrow_mut().clear());

        // First pass: Initializes history with current frame
        // Current frame has a white pixel at (5,5). History is empty (black).
        // max(current, black * 0.5) -> current.
        // So frame is unchanged, but now history has white at (5,5).
        apply_video_feedback(&mut fb, &config);

        // Frame should still have white pixel at (5, 5)
        assert_eq!(fb.get_pixel(5, 5), Some(0xFFFF_FFFF));

        // Clear the current frame to black, simulating a moving object
        fb.clear(0xFF00_0000);
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // New object

        // Second pass: the previous frame should be shifted right by 1 pixel, and blended at 50%
        apply_video_feedback(&mut fb, &config);

        // The old white pixel was at (5, 5) in history.
        // Wait, if offset_x is 0.1, the history is shifted right by 1 pixel.
        // That means to render the pixel at (6, 5), we look up history at (5, 5).
        // Since feedback_amount is 0.5, its color should be max(current, history * 0.5).
        // Current is black. History is white. White * 0.5 = 127 = 0x7F.
        let pixel = fb.get_pixel(6, 5).unwrap();
        let r = (pixel >> 16) & 0xFF;
        assert_eq!(r, 0x7F, "Expected blended pixel at (6, 5)");
    }
}
