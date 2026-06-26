//! Accumulation Motion Blur Filter.
//!
//! A post-processing effect that blends the current frame with previous frames
//! to create a sense of speed and persistence.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters and state for the Motion Blur filter.
pub struct MotionBlur {
    /// Blend factor between the history buffer and the current frame.
    /// Higher values mean more blur (history persists longer).
    /// Typically between 0.0 and 0.95.
    pub blend_factor: f32,
    /// The persistent history buffer.
    history: Vec<u32>,
    cached_width: u32,
    cached_height: u32,
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self {
            blend_factor: 0.8,
            history: Vec::new(),
            cached_width: 0,
            cached_height: 0,
        }
    }
}

impl MotionBlur {
    /// Creates a new instance of the motion blur effect with a specific blend factor.
    #[must_use]
    pub const fn new(blend_factor: f32) -> Self {
        Self {
            blend_factor: blend_factor.clamp(0.0, 1.0),
            history: Vec::new(),
            cached_width: 0,
            cached_height: 0,
        }
    }

    /// Clears the history buffer. Useful when teleporting or changing scenes.
    pub const fn clear_history(&mut self) {
        self.cached_width = 0;
        self.cached_height = 0;
    }

    /// Renders the motion blur onto the given framebuffer.
    ///
    /// * `fb`: Framebuffer to read from and render to.
    pub fn apply(&mut self, fb: &mut Framebuffer) {
        let width = fb.width();
        let height = fb.height();

        if width == 0 || height == 0 || self.blend_factor <= 0.0 {
            return;
        }

        // Initialize or resize the history buffer if framebuffer size changes
        if self.cached_width != width || self.cached_height != height {
            self.history.clear();
            self.history
                .resize((width * height) as usize, 0xFF_00_00_00);
            self.cached_width = width;
            self.cached_height = height;

            // First frame: Just copy to history directly and return
            self.history.copy_from_slice(fb.as_slice());
            return;
        }

        let blend = self.blend_factor;
        let fb_slice = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = fb_slice.par_iter_mut().zip(self.history.par_iter_mut());

        #[cfg(not(feature = "parallel"))]
        let iter = fb_slice.iter_mut().zip(self.history.iter_mut());

        iter.for_each(|(pixel, history_pixel)| {
            // Unpack ARGB
            let a1 = (*pixel >> 24) & 0xFF;
            let r1 = (*pixel >> 16) & 0xFF;
            let g1 = (*pixel >> 8) & 0xFF;
            let b1 = *pixel & 0xFF;

            let a2 = (*history_pixel >> 24) & 0xFF;
            let r2 = (*history_pixel >> 16) & 0xFF;
            let g2 = (*history_pixel >> 8) & 0xFF;
            let b2 = *history_pixel & 0xFF;

            // Perform lerp
            let new_a = (a1 as f32 * (1.0 - blend) + a2 as f32 * blend) as u32;
            let new_r = (r1 as f32 * (1.0 - blend) + r2 as f32 * blend) as u32;
            let new_g = (g1 as f32 * (1.0 - blend) + g2 as f32 * blend) as u32;
            let new_b = (b1 as f32 * (1.0 - blend) + b2 as f32 * blend) as u32;

            let blended = (new_a << 24) | (new_r << 16) | (new_g << 8) | new_b;

            // Write to framebuffer and history
            *pixel = blended;
            *history_pixel = blended;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_blur_initialization() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFFFF0000); // Red

        let mut blur = MotionBlur::new(0.5);
        blur.apply(&mut fb);

        // First frame should just prime history, no blending yet
        assert_eq!(blur.history.len(), 4);
        assert_eq!(blur.history[0], 0xFFFF0000);
        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF0000));
    }

    #[test]
    fn test_motion_blur_blending() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut blur = MotionBlur::new(0.5);

        // Frame 1: Red
        fb.set_pixel(0, 0, 0xFFFF0000);
        blur.apply(&mut fb);

        // Frame 2: Blue
        fb.set_pixel(0, 0, 0xFF0000FF);
        blur.apply(&mut fb);

        // With 0.5 blend, we expect roughly 50% red, 50% blue
        // Alpha: FF * 0.5 + FF * 0.5 = FF
        // R: 00 * 0.5 + FF * 0.5 = 7F
        // G: 00 * 0.5 + 00 * 0.5 = 00
        // B: FF * 0.5 + 00 * 0.5 = 7F
        // Result: 0xFF7F007F (or close, due to float rounding)
        let result = fb.get_pixel(0, 0).unwrap();

        let result_a = (result >> 24) & 0xFF;
        let result_r = (result >> 16) & 0xFF;
        let result_g = (result >> 8) & 0xFF;
        let result_b = result & 0xFF;

        assert_eq!(result_a, 0xFF);
        // Tolerate small rounding errors
        assert!((result_r as i32 - 0x7F).abs() <= 1, "R was {}", result_r);
        assert_eq!(result_g, 0x00);
        assert!((result_b as i32 - 0x7F).abs() <= 1, "B was {}", result_b);
    }
}
