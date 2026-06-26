//! Motion Blur Filter
//!
//! A temporal post-processing effect that maintains a history buffer
//! to simulate camera or object motion blur by blending the current frame
//! with the previous frame.

use crate::framebuffer::Framebuffer;
use std::mem;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration and state for the Motion Blur effect.
pub struct MotionBlur {
    /// History buffer containing the previous frame's pixels.
    history: Vec<u32>,
    /// The amount of motion blur feedback (0.0 to 1.0).
    /// 0.0 = no blur, 1.0 = infinite trails.
    feedback: f32,
    /// Fixed-point representation of the feedback (0-256).
    feedback_fp: u32,
}

impl MotionBlur {
    /// Creates a new `MotionBlur` instance with the given feedback strength.
    #[must_use]
    pub fn new(feedback: f32) -> Self {
        let feedback_clamped = feedback.clamp(0.0, 1.0);
        let feedback_fp = (feedback_clamped * 256.0) as u32;

        Self {
            history: Vec::new(),
            feedback: feedback_clamped,
            feedback_fp,
        }
    }

    /// Sets the feedback strength (0.0 to 1.0).
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 1.0);
        self.feedback_fp = (self.feedback * 256.0) as u32;
    }

    /// Applies the motion blur effect to the framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer) {
        if self.feedback_fp == 0 {
            // Nothing to do if there's no feedback
            return;
        }

        let pixels = fb.as_mut_slice();

        // Ensure history buffer is the correct size
        if self.history.len() != pixels.len() {
            self.history.resize(pixels.len(), 0);
            // On the first frame (or resize), just copy the current frame and return
            self.history.copy_from_slice(pixels);
            return;
        }

        let feedback = self.feedback_fp;
        let inv_feedback = 256 - feedback;

        // Blend current frame with history buffer
        #[cfg(feature = "parallel")]
        let iter = pixels.par_iter_mut().zip(self.history.par_iter_mut());

        #[cfg(not(feature = "parallel"))]
        let iter = pixels.iter_mut().zip(self.history.iter_mut());

        iter.for_each(|(pixel, hist_pixel)| {
            let p = *pixel;
            let h = *hist_pixel;

            // Extract channels for current pixel
            let a = (p >> 24) & 0xFF;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // Extract channels for history pixel
            let ha = (h >> 24) & 0xFF;
            let hr = (h >> 16) & 0xFF;
            let hg = (h >> 8) & 0xFF;
            let hb = h & 0xFF;

            // Blend
            // res = (history * feedback) + (current * (1.0 - feedback))
            let new_a = ((ha * feedback) + (a * inv_feedback)) >> 8;
            let new_r = ((hr * feedback) + (r * inv_feedback)) >> 8;
            let new_g = ((hg * feedback) + (g * inv_feedback)) >> 8;
            let new_b = ((hb * feedback) + (b * inv_feedback)) >> 8;

            let final_pixel = (new_a << 24) | (new_r << 16) | (new_g << 8) | new_b;

            // Write back to current frame and update history
            *pixel = final_pixel;
            *hist_pixel = final_pixel;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_blur_blends_frames() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let mut blur = MotionBlur::new(0.5); // 50% feedback

        // Frame 1: Solid red
        fb.clear(0xFFFF0000);
        blur.apply(&mut fb);

        // Frame 1 should be unchanged since history was empty
        assert_eq!(fb.as_slice()[0], 0xFFFF0000);

        // Frame 2: Solid blue
        fb.clear(0xFF0000FF);
        blur.apply(&mut fb);

        // With 50% feedback, the result should be a mix of red and blue.
        // We check that the red channel is not 0 and the blue channel is not 255.
        let color = fb.as_slice()[0];
        let r = (color >> 16) & 0xFF;
        let b = color & 0xFF;

        assert!(r > 0, "Red channel should be > 0 due to history blending");
        assert!(
            b < 255,
            "Blue channel should be < 255 due to history blending"
        );
    }
}
