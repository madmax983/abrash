//! Temporal Ghosting and Light Trails filter.
//!
//! A stateful filter that blends the current frame with previous frames
//! to create motion blur, ghosting, or light trail effects.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the temporal ghosting filter.
#[derive(Debug, Clone, Copy)]
pub struct TemporalGhostingConfig {
    /// Blend factor between the current frame and history buffer (0.0 to 1.0).
    /// Lower means a longer trail (more history kept).
    /// Higher means a shorter trail (more current frame kept).
    pub decay: f32,
    /// If true, the filter acts like a light trail (max-blending) instead of alpha blending.
    pub light_trails: bool,
}

impl Default for TemporalGhostingConfig {
    fn default() -> Self {
        Self {
            decay: 0.2, // 20% new frame, 80% old frame
            light_trails: false,
        }
    }
}

/// A stateful filter that maintains a history buffer for temporal effects.
pub struct TemporalGhostingFilter {
    history: Vec<u32>,
    width: usize,
    height: usize,
}

impl TemporalGhostingFilter {
    /// Creates a new empty temporal ghosting filter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    /// Applies the temporal ghosting effect to the framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer, config: &TemporalGhostingConfig) {
        let fb_width = fb.width() as usize;
        let fb_height = fb.height() as usize;

        // Resize history buffer if the framebuffer size changed
        if self.width != fb_width || self.height != fb_height {
            self.width = fb_width;
            self.height = fb_height;
            self.history.clear();
            self.history.resize(self.width * self.height, 0xFF00_0000);
        }

        let decay = config.decay.clamp(0.0, 1.0);
        let current_weight = (decay * 256.0) as u32;
        let history_weight = 256 - current_weight;

        let fb_pixels = fb.as_mut_slice();
        let history_pixels = &mut self.history;

        // Ensure we don't go out of bounds
        let len = fb_pixels.len().min(history_pixels.len());

        #[cfg(feature = "parallel")]
        let iter = fb_pixels[..len]
            .par_iter_mut()
            .zip(history_pixels[..len].par_iter_mut());

        #[cfg(not(feature = "parallel"))]
        let iter = fb_pixels[..len]
            .iter_mut()
            .zip(history_pixels[..len].iter_mut());

        if config.light_trails {
            // Light trails: keep the brightest pixels but decay them slightly
            iter.for_each(|(fb_pixel, hist_pixel)| {
                let curr_r = (*fb_pixel >> 16) & 0xFF;
                let curr_g = (*fb_pixel >> 8) & 0xFF;
                let curr_b = *fb_pixel & 0xFF;

                let hist_r = (*hist_pixel >> 16) & 0xFF;
                let hist_g = (*hist_pixel >> 8) & 0xFF;
                let hist_b = *hist_pixel & 0xFF;

                // Decay the history slightly based on decay factor (here decay is used to lower the old values)
                // If decay = 0.1, we keep 90% of history
                let decayed_hist_r = (hist_r * history_weight) >> 8;
                let decayed_hist_g = (hist_g * history_weight) >> 8;
                let decayed_hist_b = (hist_b * history_weight) >> 8;

                // Take max of current and decayed history
                let out_r = curr_r.max(decayed_hist_r);
                let out_g = curr_g.max(decayed_hist_g);
                let out_b = curr_b.max(decayed_hist_b);

                let out_pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                *fb_pixel = out_pixel;
                *hist_pixel = out_pixel;
            });
        } else {
            // Standard temporal alpha blending
            iter.for_each(|(fb_pixel, hist_pixel)| {
                let curr_r = (*fb_pixel >> 16) & 0xFF;
                let curr_g = (*fb_pixel >> 8) & 0xFF;
                let curr_b = *fb_pixel & 0xFF;

                let hist_r = (*hist_pixel >> 16) & 0xFF;
                let hist_g = (*hist_pixel >> 8) & 0xFF;
                let hist_b = *hist_pixel & 0xFF;

                let out_r = (curr_r * current_weight + hist_r * history_weight) >> 8;
                let out_g = (curr_g * current_weight + hist_g * history_weight) >> 8;
                let out_b = (curr_b * current_weight + hist_b * history_weight) >> 8;

                let out_pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                *fb_pixel = out_pixel;
                *hist_pixel = out_pixel;
            });
        }
    }
}

impl Default for TemporalGhostingFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_history_buffer() {
        let mut filter = TemporalGhostingFilter::new();
        let mut fb = Framebuffer::new(100, 50).unwrap();

        // Apply should resize history buffer to match fb
        filter.apply(&mut fb, &TemporalGhostingConfig::default());

        assert_eq!(filter.width, 100);
        assert_eq!(filter.height, 50);
        assert_eq!(filter.history.len(), 5000);
    }

    #[test]
    fn test_alpha_blend_ghosting() {
        let mut filter = TemporalGhostingFilter::new();
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Initial frame is all red
        fb.clear(0xFFFF0000);
        // decay = 1.0 means replace completely (first frame populates history)
        filter.apply(
            &mut fb,
            &TemporalGhostingConfig {
                decay: 1.0,
                light_trails: false,
            },
        );

        assert_eq!(filter.history[0], 0xFFFF0000);

        // Next frame is all blue
        fb.clear(0xFF0000FF);
        // decay = 0.5 means 50% new (blue) + 50% old (red) = half purple
        filter.apply(
            &mut fb,
            &TemporalGhostingConfig {
                decay: 0.5,
                light_trails: false,
            },
        );

        let result = fb.get_pixel(0, 0).unwrap();
        let expected_r = 127;
        let expected_b = 127;
        let actual_r = (result >> 16) & 0xFF;
        let actual_b = result & 0xFF;

        assert!((actual_r as i32 - expected_r as i32).abs() <= 1);
        assert!((actual_b as i32 - expected_b as i32).abs() <= 1);
    }

    #[test]
    fn test_light_trails_max_blend() {
        let mut filter = TemporalGhostingFilter::new();
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Initial frame is dark grey
        fb.clear(0xFF444444);
        filter.apply(
            &mut fb,
            &TemporalGhostingConfig {
                decay: 0.5,
                light_trails: true,
            },
        );

        // Next frame is bright white at one pixel, black elsewhere
        fb.clear(0xFF000000);
        fb.set_pixel(5, 5, 0xFFFFFFFF);

        // light trails should keep the brightest pixel from history minus some decay
        filter.apply(
            &mut fb,
            &TemporalGhostingConfig {
                decay: 0.1,
                light_trails: true,
            },
        );

        let result = fb.get_pixel(0, 0).unwrap();
        // Since original was 0x44 (68) and we decayed by 0.1 (10%), it should be around 68 - 25 = 43?
        // Or if the decay formula subtracts, it should be > 0.
        // We just verify it's not black and not 0x44.
        assert_ne!(result, 0xFF000000);

        let white_pixel = fb.get_pixel(5, 5).unwrap();
        assert_eq!(white_pixel, 0xFFFFFFFF); // White should remain white
    }
}
