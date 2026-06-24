//! Temporal Motion Blur Post-Processing Filter
//!
//! Blends the current frame with previous frames to create a motion blur effect.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Motion Blur effect.
#[derive(Debug, Clone, Copy)]
pub struct MotionBlurConfig {
    /// The blend factor for the new frame. 0.0 means only the old frame is kept, 1.0 means no blur.
    pub blend_factor: f32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self { blend_factor: 0.5 }
    }
}

/// A stateful motion blur processor that maintains the temporal accumulator buffer.
pub struct MotionBlur {
    accumulator: Vec<u32>,
}

impl MotionBlur {
    /// Creates a new, uninitialized motion blur processor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            accumulator: Vec::new(),
        }
    }

    /// Applies a temporal motion blur effect to the framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer, config: MotionBlurConfig) {
        let width = fb.width();
        let height = fb.height();
        let buffer_size = (width * height) as usize;

        // Initialize or resize the accumulator buffer if necessary.
        if self.accumulator.len() != buffer_size {
            self.accumulator.clear();
            self.accumulator.extend_from_slice(fb.as_slice());
            return; // First frame, just copy and exit
        }

        let blend_factor = config.blend_factor.clamp(0.0, 1.0);
        let one_minus_blend = 1.0 - blend_factor;

        let fb_buffer = fb.as_mut_slice();

        for (i, current_pixel) in fb_buffer.iter_mut().enumerate() {
            let old_pixel = self.accumulator[i];

            let r_old = ((old_pixel >> 16) & 0xFF) as f32;
            let g_old = ((old_pixel >> 8) & 0xFF) as f32;
            let b_old = (old_pixel & 0xFF) as f32;

            let r_new = ((*current_pixel >> 16) & 0xFF) as f32;
            let g_new = ((*current_pixel >> 8) & 0xFF) as f32;
            let b_new = (*current_pixel & 0xFF) as f32;

            let r_blended = (r_old * one_minus_blend + r_new * blend_factor) as u32;
            let g_blended = (g_old * one_minus_blend + g_new * blend_factor) as u32;
            let b_blended = (b_old * one_minus_blend + b_new * blend_factor) as u32;

            let blended_pixel = 0xFF00_0000 | (r_blended << 16) | (g_blended << 8) | b_blended;

            *current_pixel = blended_pixel;
            self.accumulator[i] = blended_pixel;
        }
    }
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_blur_initialization() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF111111);

        let mut filter = MotionBlur::new();
        filter.apply(&mut fb, MotionBlurConfig { blend_factor: 0.5 });

        // On first frame, output should match input exactly
        assert_eq!(fb.as_slice()[0], 0xFF111111);
    }

    #[test]
    fn test_motion_blur_blending() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let mut filter = MotionBlur::new();

        // First frame to populate accumulator
        fb.clear(0xFF000000);
        filter.apply(&mut fb, MotionBlurConfig { blend_factor: 0.5 });

        // Second frame: blend black with white
        fb.clear(0xFFFFFFFF);
        filter.apply(&mut fb, MotionBlurConfig { blend_factor: 0.5 });

        // Expect gray (around 0xFF7F7F7F or 0xFF808080)
        let color = fb.as_slice()[0];
        let r = (color >> 16) & 0xFF;
        assert!(
            r > 0x70 && r < 0x90,
            "Expected gray color, got 0x{:08X}",
            color
        );
    }
}
