//! Phosphor Decay (CRT Ghosting) Filter
//!
//! Simulates the temporal persistence of phosphors on old CRT monitors.
//! Different color channels (like green) often decay slower than others,
//! leaving colorful trails behind moving objects.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A stateful filter that simulates phosphor decay (CRT ghosting).
///
/// It maintains a historical buffer to blend consecutive frames,
/// producing a trail effect behind moving objects.
pub struct PhosphorDecayFilter {
    history: Vec<u32>,
}

impl Default for PhosphorDecayFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PhosphorDecayFilter {
    /// Creates a new, empty Phosphor Decay filter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    /// Applies the phosphor decay effect to the framebuffer.
    ///
    /// Blends the current frame with the historical buffer.
    pub fn apply(&mut self, fb: &mut Framebuffer, config: &PhosphorDecayConfig) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let size = width * height;

        if size == 0 {
            return;
        }

        // Ensure the history buffer is sized correctly for the current framebuffer.
        if self.history.len() != size {
            self.history.clear();
            self.history.resize(size, 0xFF00_0000);
        }

        let hist_slice = &mut self.history[..];
        let fb_slice = fb.as_mut_slice();

        // Convert 0.0-1.0 decay rates to fixed-point multipliers (0-256)
        let decay_r_fixed = ((1.0 - config.decay_r.clamp(0.0, 1.0)) * 256.0) as u32;
        let decay_g_fixed = ((1.0 - config.decay_g.clamp(0.0, 1.0)) * 256.0) as u32;
        let decay_b_fixed = ((1.0 - config.decay_b.clamp(0.0, 1.0)) * 256.0) as u32;

        #[cfg(feature = "parallel")]
        let iter = fb_slice.par_iter_mut().zip(hist_slice.par_iter_mut());

        #[cfg(not(feature = "parallel"))]
        let iter = fb_slice.iter_mut().zip(hist_slice.iter_mut());

        iter.for_each(|(pixel, hist_pixel)| {
            let p = *pixel;
            let hp = *hist_pixel;

            let p_r = (p >> 16) & 0xFF;
            let p_g = (p >> 8) & 0xFF;
            let p_b = p & 0xFF;

            let hp_r = (hp >> 16) & 0xFF;
            let hp_g = (hp >> 8) & 0xFF;
            let hp_b = hp & 0xFF;

            // Apply decay to historical pixel
            let decayed_r = (hp_r * decay_r_fixed) >> 8;
            let decayed_g = (hp_g * decay_g_fixed) >> 8;
            let decayed_b = (hp_b * decay_b_fixed) >> 8;

            // The new pixel value is the maximum of the current frame and the decayed history.
            // This mimics phosphors lighting up instantly but fading slowly.
            let out_r = p_r.max(decayed_r);
            let out_g = p_g.max(decayed_g);
            let out_b = p_b.max(decayed_b);

            let result = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;

            *pixel = result;
            *hist_pixel = result;
        });
    }
}

/// Configuration for the Phosphor Decay filter.
#[derive(Debug, Clone, Copy)]
pub struct PhosphorDecayConfig {
    /// Decay rate for the Red channel (0.0 = infinite persistence, 1.0 = instant decay).
    pub decay_r: f32,
    /// Decay rate for the Green channel (0.0 = infinite persistence, 1.0 = instant decay).
    pub decay_g: f32,
    /// Decay rate for the Blue channel (0.0 = infinite persistence, 1.0 = instant decay).
    pub decay_b: f32,
}

impl Default for PhosphorDecayConfig {
    fn default() -> Self {
        Self {
            decay_r: 0.3,
            decay_g: 0.1, // Green decays slowest on many CRTs
            decay_b: 0.4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_phosphor_decay() {
        let width = 5;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();

        let config = PhosphorDecayConfig {
            decay_r: 0.5,
            decay_g: 0.2, // Green decays slower
            decay_b: 0.8, // Blue decays faster
        };

        let mut filter = PhosphorDecayFilter::new();

        // Frame 1: Draw a bright white pixel at (0, 0)
        fb.clear(0xFF00_0000);
        fb.set_pixel(0, 0, 0xFFFF_FFFF);
        filter.apply(&mut fb, &config);

        // Frame 2: Pixel moves to (1, 0), original spot should now be a ghost trail
        fb.clear(0xFF00_0000);
        fb.set_pixel(1, 0, 0xFFFF_FFFF);
        filter.apply(&mut fb, &config);

        // The pixel at (0,0) should have decayed, but not be completely black
        let ghost_pixel = fb.get_pixel(0, 0).unwrap();

        let ghost_r = (ghost_pixel >> 16) & 0xFF;
        let ghost_g = (ghost_pixel >> 8) & 0xFF;
        let ghost_b = ghost_pixel & 0xFF;

        assert!(
            ghost_pixel != 0xFF00_0000,
            "Ghost trail should exist at (0,0)"
        );

        // Since Green decays slower, it should be the brightest component of the trail
        assert!(ghost_g > ghost_r, "Green trail should be brighter than Red");
        assert!(ghost_r > ghost_b, "Red trail should be brighter than Blue");
    }
}
