//! Video Feedback / Hall of Mirrors Post-Processing Effect
//!
//! Simulates analog video feedback (pointing a camera at its own monitor) by
//! maintaining a persistent history buffer and blending the current frame
//! with a transformed (scaled, rotated, faded) version of the previous frame.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static HISTORY_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Video Feedback effect.
#[derive(Debug, Clone, Copy)]
pub struct VideoFeedbackConfig {
    /// How much the previous frame should be scaled (e.g., 0.95 for zooming in, 1.05 for zooming out).
    pub scale: f32,
    /// How much the previous frame should be rotated per frame, in radians.
    pub rotation: f32,
    /// How much the previous frame's colors should decay/fade (0.0 = disappear instantly, 1.0 = never fade).
    pub decay: f32,
    /// Offset X to shift the feedback center relative to screen center.
    pub offset_x: f32,
    /// Offset Y to shift the feedback center relative to screen center.
    pub offset_y: f32,
}

impl Default for VideoFeedbackConfig {
    fn default() -> Self {
        Self {
            scale: 0.95,
            rotation: 0.05,
            decay: 0.9,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Applies the video feedback effect to the framebuffer.
///
/// Modifies the given framebuffer in-place by blending it with a transformed
/// (scaled, rotated, faded) version of the previous frame. It saves the newly
/// blended result internally to be used as the history for the next call.
///
/// If `clear_history` is true, the internal history buffer will be cleared,
/// effectively restarting the effect. This is useful when resizing or switching scenes.
///
/// # Arguments
///
/// * `fb` - The current framebuffer to process.
/// * `config` - The configuration controlling the feedback transformation.
/// * `clear_history` - Set to `true` to clear the internal buffer.
pub fn apply_video_feedback(
    fb: &mut Framebuffer,
    config: &VideoFeedbackConfig,
    clear_history: bool,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let size = width * height;

    if size == 0 {
        return;
    }

    HISTORY_BUFFER.with(|buf| {
        let mut history = buf.borrow_mut();

        if clear_history || history.len() != size {
            history.resize(size, 0xFF00_0000);
            history.fill(0xFF00_0000);
        }

        if config.decay <= 0.0 {
            // If decay is 0, just save the current frame and return
            history.copy_from_slice(fb.as_slice());
            return;
        }

        let cx = (width as f32 / 2.0) + config.offset_x;
        let cy = (height as f32 / 2.0) + config.offset_y;

        let (sin_a, cos_a) = config.rotation.sin_cos();
        let scale_inv = if config.scale == 0.0 {
            1.0
        } else {
            1.0 / config.scale
        };

        let decay_i32 = (config.decay.clamp(0.0, 1.0) * 255.0) as i32;

        // Create a temporary buffer to hold the transformed history
        let mut transformed_history = vec![0xFF00_0000; size];
        let history_slice = history.as_slice();

        // 1. Transform the history frame
        #[cfg(feature = "parallel")]
        {
            transformed_history
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    let dy = y as f32 - cy;
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx;

                        // Inverse transform to find the source pixel in the history buffer
                        // To zoom *in* (scale > 1.0), we need to sample from closer to the center, so multiply by scale_inv
                        let src_dx = (dx * cos_a + dy * sin_a) * scale_inv;
                        let src_dy = (-dx * sin_a + dy * cos_a) * scale_inv;

                        // Cast to integer coordinates
                        // ⚡ Bolt: Replace f32::round() with fast integer casting offset by 0.5
                        let src_x = (src_dx + cx + 0.5) as i32;
                        let src_y = (src_dy + cy + 0.5) as i32;

                        if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32
                        {
                            let src_idx = (src_y as usize) * width + (src_x as usize);
                            *pixel = history_slice[src_idx];
                        }
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let dy = y as f32 - cy;
                for x in 0..width {
                    let dx = x as f32 - cx;

                    let src_dx = (dx * cos_a + dy * sin_a) * scale_inv;
                    let src_dy = (-dx * sin_a + dy * cos_a) * scale_inv;

                    let src_x = (src_dx + cx + 0.5) as i32;
                    let src_y = (src_dy + cy + 0.5) as i32;

                    let dest_idx = y * width + x;

                    if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                        let src_idx = (src_y as usize) * width + (src_x as usize);
                        transformed_history[dest_idx] = history_slice[src_idx];
                    }
                }
            }
        }

        // 2. Blend the current frame with the transformed history
        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            pixels
                .par_iter_mut()
                .zip(transformed_history.par_iter())
                .for_each(|(curr_pixel, hist_pixel)| {
                    blend_pixels(curr_pixel, *hist_pixel, decay_i32);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for (curr_pixel, hist_pixel) in pixels.iter_mut().zip(transformed_history.iter()) {
                blend_pixels(curr_pixel, *hist_pixel, decay_i32);
            }
        }

        // 3. Save the result back to history
        history.copy_from_slice(fb.as_slice());
    });
}

#[inline(always)]
fn blend_pixels(curr_pixel: &mut u32, hist_pixel: u32, decay_i32: i32) {
    let c = *curr_pixel;
    let h = hist_pixel;

    // Apply additive blending (Screen-like or simple Additive max)
    // We'll decay the history and then take the max of current and history
    // This allows bright objects moving to leave fading trails.

    let hr = (((h >> 16) & 0xFF) as i32 * decay_i32) >> 8;
    let hg = (((h >> 8) & 0xFF) as i32 * decay_i32) >> 8;
    let hb = ((h & 0xFF) as i32 * decay_i32) >> 8;

    let cr = ((c >> 16) & 0xFF) as i32;
    let cg = ((c >> 8) & 0xFF) as i32;
    let cb = (c & 0xFF) as i32;

    // Lighten / Max blend
    let fr = cr.max(hr).min(255) as u32;
    let fg = cg.max(hg).min(255) as u32;
    let fb = cb.max(hb).min(255) as u32;

    *curr_pixel = 0xFF00_0000 | (fr << 16) | (fg << 8) | fb;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_video_feedback_no_decay() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF12_3456);

        let config = VideoFeedbackConfig {
            decay: 0.0,
            ..Default::default()
        };

        apply_video_feedback(&mut fb, &config, true);

        for &p in fb.as_slice() {
            assert_eq!(p, 0xFF12_3456);
        }
    }

    #[test]
    fn test_apply_video_feedback_changes_buffer() {
        let width = 20;
        let height = 20;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Frame 1: White pixel in the center
        fb.clear(0xFF00_0000);
        fb.set_pixel(10, 10, 0xFFFF_FFFF);

        let config = VideoFeedbackConfig {
            scale: 1.1,
            rotation: 0.0,
            decay: 0.5,
            offset_x: 0.0,
            offset_y: 0.0,
        };

        // Call it once to prime the history
        apply_video_feedback(&mut fb, &config, true);

        // Frame 2: Black framebuffer, should pull in faded white pixel from history
        fb.clear(0xFF00_0000);
        apply_video_feedback(&mut fb, &config, false);

        // Verify that the buffer is not completely black due to feedback
        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF00_0000 {
                has_non_black = true;
                break;
            }
        }

        assert!(
            has_non_black,
            "Video feedback did not apply history to the framebuffer"
        );
    }
}
