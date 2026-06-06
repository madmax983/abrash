//! Video Feedback Post-Processing Filter
//!
//! A retro effect that simulates an analog video feedback loop
//! by feeding the previous frame back into the current frame with
//! optional transformations (scale, rotation, translation) and decay.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Video Feedback effect.
#[derive(Debug, Clone, Copy)]
pub struct VideoFeedbackConfig {
    /// How much the previous frame decays per frame (0.0 to 1.0).
    /// 1.0 means no decay (infinite trail), 0.0 means no trail.
    pub decay: f32,
    /// Scale factor applied to the feedback frame.
    /// > 1.0 creates an outward tunneling effect.
    /// < 1.0 creates an inward collapsing effect.
    pub scale: f32,
    /// Rotation applied to the feedback frame (in radians).
    pub rotation: f32,
    /// Horizontal offset applied to the feedback frame.
    pub offset_x: f32,
    /// Vertical offset applied to the feedback frame.
    pub offset_y: f32,
}

impl Default for VideoFeedbackConfig {
    fn default() -> Self {
        Self {
            decay: 0.95,
            scale: 1.01,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// State for the Video Feedback effect.
pub struct VideoFeedback {
    history: Vec<u32>,
    history_snapshot: Vec<u32>,
}

impl Default for VideoFeedback {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoFeedback {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: Vec::new(),
            history_snapshot: Vec::new(),
        }
    }

    /// Applies a video feedback effect to the framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer, config: &VideoFeedbackConfig) {
        if config.decay <= 0.0 {
            return;
        }

        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let num_pixels = width * height;

        if num_pixels == 0 {
            return;
        }

        if self.history.len() != num_pixels {
            self.history.resize(num_pixels, 0);
            self.history_snapshot.resize(num_pixels, 0);
        }

        // We use an explicit snapshot buffer instead of cloning the vector on every frame.
        self.history_snapshot.copy_from_slice(&self.history);
        let history_snapshot = &self.history_snapshot;

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        let inv_scale = if config.scale == 0.0 {
            0.0
        } else {
            1.0 / config.scale
        };
        let cos_theta = (-config.rotation).cos();
        let sin_theta = (-config.rotation).sin();

        let width_i64 = width as i64;
        let height_i64 = height as i64;

        #[cfg(not(feature = "parallel"))]
        let chunk_iter = fb.as_mut_slice().chunks_exact_mut(width);
        #[cfg(feature = "parallel")]
        let chunk_iter = fb.as_mut_slice().par_chunks_exact_mut(width);

        chunk_iter.enumerate().for_each(|(y, row)| {
            let dy = y as f32 - cy;
            let dy_offset = dy - config.offset_y;

            for x in 0..width {
                let dx = x as f32 - cx;
                let dx_offset = dx - config.offset_x;

                let rx = dx_offset * cos_theta - dy_offset * sin_theta;
                let ry = dx_offset * sin_theta + dy_offset * cos_theta;

                let hx = rx * inv_scale + cx;
                let hy = ry * inv_scale + cy;

                let hx_i = hx.round() as i64;
                let hy_i = hy.round() as i64;

                let hist_color = if hx_i >= 0 && hx_i < width_i64 && hy_i >= 0 && hy_i < height_i64
                {
                    let hist_idx = (hy_i * width_i64 + hx_i) as usize;
                    history_snapshot[hist_idx]
                } else {
                    0x0000_0000
                };

                let curr_c = row[x];
                let curr_a = ((curr_c >> 24) & 0xFF) as f32;
                let curr_r = ((curr_c >> 16) & 0xFF) as f32;
                let curr_g = ((curr_c >> 8) & 0xFF) as f32;
                let curr_b = (curr_c & 0xFF) as f32;

                let hist_a = ((hist_color >> 24) & 0xFF) as f32;
                let hist_r = ((hist_color >> 16) & 0xFF) as f32;
                let hist_g = ((hist_color >> 8) & 0xFF) as f32;
                let hist_b = (hist_color & 0xFF) as f32;

                let dec_a = hist_a * config.decay;
                let dec_r = hist_r * config.decay;
                let dec_g = hist_g * config.decay;
                let dec_b = hist_b * config.decay;

                let out_a = (curr_a + dec_a).min(255.0) as u32;
                let out_r = (curr_r + dec_r).min(255.0) as u32;
                let out_g = (curr_g + dec_g).min(255.0) as u32;
                let out_b = (curr_b + dec_b).min(255.0) as u32;

                row[x] = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
            }
        });

        self.history.copy_from_slice(fb.as_slice());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_feedback_decay() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF_FF_FF_FF);

        let config = VideoFeedbackConfig {
            decay: 0.5,
            scale: 1.0,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
        };

        let mut vf = VideoFeedback::new();

        vf.apply(&mut fb, &config);

        fb.clear(0xFF_00_00_00);
        vf.apply(&mut fb, &config);

        let pixel = fb.get_pixel(0, 0).unwrap();

        let expected = 0xFF_7F_7F_7F;
        assert_eq!(pixel, expected, "Expected decayed white blended with black");
    }
}
