use crate::framebuffer::Framebuffer;

/// Configuration for the Motion Blur effect.
#[derive(Debug, Clone, Copy)]
pub struct MotionBlurConfig {
    /// The blend factor between the previous frame and the current frame (0.0 to 1.0).
    /// 0.0 means no motion blur (only current frame).
    /// 1.0 means full motion blur (only previous frame - will cause screen to freeze).
    /// Typically values like 0.7 to 0.9 produce a nice trailing effect.
    pub blend_factor: f32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            blend_factor: 0.8,
        }
    }
}

/// A stateful post-processing effect that applies temporal motion blur.
///
/// This blends the current framebuffer with the previous frame's framebuffer
/// to create a ghosting or trailing effect on moving objects.
pub struct MotionBlurEffect {
    previous_frame: Vec<u32>,
    config: MotionBlurConfig,
}

impl MotionBlurEffect {
    /// Creates a new `MotionBlurEffect` with the given configuration.
    #[must_use]
    pub const fn new(config: MotionBlurConfig) -> Self {
        Self {
            previous_frame: Vec::new(),
            config,
        }
    }

    /// Applies the motion blur effect to the given framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let len = width * height;

        if len == 0 {
            return;
        }

        let blend = self.config.blend_factor.clamp(0.0, 1.0);
        // Integer math: 0-256 for faster blending
        let blend_int = (blend * 256.0) as u32;
        let inv_blend_int = 256 - blend_int;

        let dest_pixels = fb.as_mut_slice();

        // Initialize if first frame or resized
        if self.previous_frame.len() != len {
            self.previous_frame.resize(len, 0xFF00_0000);
            // On first frame, just copy the current frame and return
            self.previous_frame.copy_from_slice(dest_pixels);
            return;
        }

        let prev_slice = self.previous_frame.as_mut_slice();

        // Blend current frame with previous frame
        for (i, current) in dest_pixels.iter_mut().enumerate() {
            let prev = prev_slice[i];

            let a_curr = (*current >> 24) & 0xFF;
            let r_curr = (*current >> 16) & 0xFF;
            let g_curr = (*current >> 8) & 0xFF;
            let b_curr = *current & 0xFF;

            let a_prev = (prev >> 24) & 0xFF;
            let r_prev = (prev >> 16) & 0xFF;
            let g_prev = (prev >> 8) & 0xFF;
            let b_prev = prev & 0xFF;

            let a = ((a_prev * blend_int + a_curr * inv_blend_int) >> 8) & 0xFF;
            let r = ((r_prev * blend_int + r_curr * inv_blend_int) >> 8) & 0xFF;
            let g = ((g_prev * blend_int + g_curr * inv_blend_int) >> 8) & 0xFF;
            let b = ((b_prev * blend_int + b_curr * inv_blend_int) >> 8) & 0xFF;

            let blended = (a << 24) | (r << 16) | (g << 8) | b;

            *current = blended;
            prev_slice[i] = blended;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_blur_initialization() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_0000); // Red

        let config = MotionBlurConfig { blend_factor: 0.5 };
        let mut effect = MotionBlurEffect::new(config);

        // First call should just initialize
        effect.apply(&mut fb);
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_0000);

        // Change framebuffer to blue
        fb.clear(0xFF00_00FF);

        // Second call should blend (50% red, 50% blue -> purple)
        effect.apply(&mut fb);

        let blended = fb.get_pixel(0, 0).unwrap();
        let r = (blended >> 16) & 0xFF;
        let b = blended & 0xFF;

        assert!(r > 0 && r < 255);
        assert!(b > 0 && b < 255);
    }
}
