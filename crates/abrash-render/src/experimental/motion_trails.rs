//! Motion Trails Post-Processing Filter
//!
//! A temporal post-processing effect that blends the current frame with a fading
//! history of previous frames, creating a "ghosting" or motion trail effect.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Motion Trails effect.
#[derive(Debug, Clone, Copy)]
pub struct MotionTrailsConfig {
    /// How much of the previous frame should be retained (0.0 to 1.0).
    /// Higher values mean longer trails.
    pub decay: f32,
}

impl Default for MotionTrailsConfig {
    fn default() -> Self {
        Self { decay: 0.8 }
    }
}

/// Applies a Motion Trails effect to the framebuffer.
///
/// This blends the current framebuffer with the history buffer, and then updates
/// the history buffer. The caller is responsible for maintaining the `history_buffer`
/// between frames to ensure proper temporal blending.
pub fn apply_motion_trails(
    fb: &mut Framebuffer,
    history_buffer: &mut Vec<u32>,
    config: &MotionTrailsConfig,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let decay_factor = config.decay.clamp(0.0, 1.0);
    let inv_decay = 1.0 - decay_factor;

    let size = width * height;

    // Initialize history buffer if empty or resized
    if history_buffer.len() != size {
        history_buffer.resize(size, 0xFF00_0000); // Black with full alpha
        // First frame just seeds the buffer, no trails yet.
        history_buffer.copy_from_slice(fb.as_slice());
        return;
    }

    let history_pixels = &mut history_buffer[..size];
    let current_pixels = fb.as_mut_slice();

    // Blend current frame with history
    #[cfg(feature = "parallel")]
    let iter = current_pixels
        .par_iter_mut()
        .zip(history_pixels.par_iter_mut());
    #[cfg(not(feature = "parallel"))]
    let iter = current_pixels.iter_mut().zip(history_pixels.iter_mut());

    iter.for_each(|(curr, hist)| {
        let c_r = ((*curr >> 16) & 0xFF) as f32;
        let c_g = ((*curr >> 8) & 0xFF) as f32;
        let c_b = (*curr & 0xFF) as f32;

        let h_r = ((*hist >> 16) & 0xFF) as f32;
        let h_g = ((*hist >> 8) & 0xFF) as f32;
        let h_b = (*hist & 0xFF) as f32;

        let final_r = (c_r * inv_decay + h_r * decay_factor).clamp(0.0, 255.0) as u32;
        let final_g = (c_g * inv_decay + h_g * decay_factor).clamp(0.0, 255.0) as u32;
        let final_b = (c_b * inv_decay + h_b * decay_factor).clamp(0.0, 255.0) as u32;

        let result = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;

        // Output to framebuffer
        *curr = result;
        // Update history for next frame
        *hist = result;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_motion_trails() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Initialize History buffer by running once
        let config = MotionTrailsConfig { decay: 0.5 };
        let mut history = Vec::new();

        // Frame 1: Pure White
        fb.clear(0xFF_FF_FF_FF);
        apply_motion_trails(&mut fb, &mut history, &config);

        // Output should be white since history was empty, now history is white
        assert_eq!(fb.get_pixel(5, 5).unwrap(), 0xFF_FF_FF_FF);

        // Frame 2: Pure Black
        fb.clear(0xFF_00_00_00);
        apply_motion_trails(&mut fb, &mut history, &config);

        // Output should be 50% white (127), as decay is 0.5
        let p = fb.get_pixel(5, 5).unwrap();
        let r = (p >> 16) & 0xFF;
        assert!(r >= 126 && r <= 128, "Expected ~127, got {r}");
    }
}
