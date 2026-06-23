//! Phosphor Decay (Ghosting) Filter
//!
//! Simulates the persistence of vision and the slow decay of phosphors on
//! vintage CRT monitors or early LCD panels, creating a ghosting trail behind moving objects.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static ACCUMULATOR: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Phosphor Decay (Ghosting) effect.
#[derive(Debug, Clone, Copy)]
pub struct PhosphorDecayConfig {
    /// Decay rate of the phosphor trails (0.0 to 1.0).
    /// 0.0 = Instant decay (no trails), 0.99 = Very long trails.
    pub decay_factor: f32,
    /// If true, uses additive 'light' blending (retains brightness).
    /// If false, uses standard lerp.
    pub luminous: bool,
}

impl Default for PhosphorDecayConfig {
    fn default() -> Self {
        Self {
            decay_factor: 0.8,
            luminous: true,
        }
    }
}

/// Applies a phosphor decay/ghosting effect to the framebuffer.
///
/// Simulates the slow decay of phosphors by accumulating frames and gradually
/// fading them out over time. It stores the accumulator buffer in a thread-local.
pub fn apply_phosphor_decay(fb: &mut Framebuffer, config: &PhosphorDecayConfig) {
    if config.decay_factor <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let size = width * height;

    if size == 0 {
        return;
    }

    let decay_fixed = (config.decay_factor.clamp(0.0, 1.0) * 256.0) as u32;
    let inv_decay_fixed = 256u32.saturating_sub(decay_fixed);

    let pixels = fb.as_mut_slice();

    ACCUMULATOR.with(|acc| {
        let mut accum_buffer = acc.borrow_mut();

        // Handle resize or initialization
        if accum_buffer.len() != size {
            accum_buffer.resize(size, 0xFF_000000);
        }

        let accum_slice = &mut accum_buffer[..size];

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            pixels
                .par_iter_mut()
                .zip(accum_slice.par_iter_mut())
                .for_each(|(p, a)| {
                    process_pixel(p, a, decay_fixed, inv_decay_fixed, config.luminous);
                });
        }
        #[cfg(not(feature = "parallel"))]
        {
            for (p, a) in pixels.iter_mut().zip(accum_slice.iter_mut()) {
                process_pixel(p, a, decay_fixed, inv_decay_fixed, config.luminous);
            }
        }
    });
}

#[inline(always)]
fn process_pixel(p: &mut u32, a: &mut u32, decay: u32, inv_decay: u32, luminous: bool) {
    let curr = *p;
    let prev = *a;

    let cr = (curr >> 16) & 0xFF;
    let cg = (curr >> 8) & 0xFF;
    let cb = curr & 0xFF;

    let pr = (prev >> 16) & 0xFF;
    let pg = (prev >> 8) & 0xFF;
    let pb = prev & 0xFF;

    let (nr, ng, nb) = if luminous {
        // Luminous mode: Max of current and decayed previous (good for neon/light sources)
        (
            cr.max((pr * decay) >> 8),
            cg.max((pg * decay) >> 8),
            cb.max((pb * decay) >> 8),
        )
    } else {
        // Standard blend mode: mix current and previous
        (
            ((cr * inv_decay) + (pr * decay)) >> 8,
            ((cg * inv_decay) + (pg * decay)) >> 8,
            ((cb * inv_decay) + (pb * decay)) >> 8,
        )
    };

    let new_pixel = 0xFF_000000 | (nr << 16) | (ng << 8) | nb;
    *p = new_pixel;
    *a = new_pixel; // Store for next frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phosphor_decay_leaves_trail() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        let config = PhosphorDecayConfig {
            decay_factor: 0.5,
            luminous: true,
        };

        // Frame 1: Bright pixel at (2, 2)
        fb.clear(0xFF_000000);
        fb.set_pixel(2, 2, 0xFF_FFFFFF);
        apply_phosphor_decay(&mut fb, &config);
        assert_eq!(fb.get_pixel(2, 2).unwrap(), 0xFF_FFFFFF);

        // Frame 2: Bright pixel moves to (3, 2), original position (2, 2) is drawn black
        fb.clear(0xFF_000000);
        fb.set_pixel(3, 2, 0xFF_FFFFFF);
        apply_phosphor_decay(&mut fb, &config);

        // The new position should be bright
        assert_eq!(fb.get_pixel(3, 2).unwrap(), 0xFF_FFFFFF);

        // The old position (2, 2) should NOT be black anymore. It should have decayed.
        // It was 0xFF, decay is 0.5, so it should be around 0x80 (128).
        let old_pixel = fb.get_pixel(2, 2).unwrap();
        let old_r = (old_pixel >> 16) & 0xFF;
        assert!(
            old_r > 0 && old_r < 255,
            "Trail was not left behind at old position"
        );

        // Clear state for other tests by resetting decay buffer length
        ACCUMULATOR.with(|acc| acc.borrow_mut().clear());
    }

    #[test]
    fn test_phosphor_decay_zero_decay() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let config = PhosphorDecayConfig {
            decay_factor: 0.0,
            luminous: true,
        };

        fb.clear(0xFF_000000);
        fb.set_pixel(0, 0, 0xFF_FFFFFF);
        apply_phosphor_decay(&mut fb, &config);

        fb.clear(0xFF_000000);
        fb.set_pixel(1, 1, 0xFF_FFFFFF);
        apply_phosphor_decay(&mut fb, &config);

        // With zero decay, the old position should be completely black
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_000000);

        ACCUMULATOR.with(|acc| acc.borrow_mut().clear());
    }
}
