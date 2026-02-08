//! Post-processing effects for the framebuffer.
//!
//! Provides a trait and implementations for applying visual effects
//! to the rendered image.

use crate::framebuffer::Framebuffer;

/// Trait for applying a post-processing effect.
pub trait PostProcess {
    /// Applies the effect to the given framebuffer in-place.
    fn apply(&self, buffer: &mut Framebuffer);
}

/// Converts the image to grayscale using luminance weights.
pub struct Grayscale;

impl PostProcess for Grayscale {
    fn apply(&self, buffer: &mut Framebuffer) {
        let pixels = buffer.as_mut_slice();
        for pixel in pixels.iter_mut() {
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;

            // Standard luminance weights: 0.299 R + 0.587 G + 0.114 B
            // Using integer math for speed: (299*R + 587*G + 114*B) / 1000
            let y = (299 * r + 587 * g + 114 * b) / 1000;

            *pixel = 0xFF00_0000 | (y << 16) | (y << 8) | y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grayscale() {
        let mut buffer = Framebuffer::new(2, 2).unwrap();
        // Fully Red: 0.299 -> ~76 (0x4C)
        buffer.set_pixel(0, 0, 0xFFFF_0000);

        let effect = Grayscale;
        effect.apply(&mut buffer);

        let pixel = buffer.get_pixel(0, 0).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert_eq!(r, g);
        assert_eq!(g, b);
        // Allow small rounding diffs
        assert!((r as i32 - 76).abs() <= 1);
    }

    #[test]
    fn test_invert() {
        let mut buffer = Framebuffer::new(1, 1).unwrap();
        buffer.set_pixel(0, 0, 0xFF00_0000); // Black

        let effect = Invert;
        effect.apply(&mut buffer);

        assert_eq!(buffer.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF); // White
    }

    #[test]
    fn test_scanlines() {
        let mut buffer = Framebuffer::new(1, 4).unwrap();
        buffer.clear(0xFFFF_FFFF); // White

        // Darken every 2nd line by 50%
        // Spacing 2: Lines 0, 2 affected. Lines 1, 3 untouched.
        let effect = Scanlines::new(0.5, 2);
        effect.apply(&mut buffer);

        // Line 0: Darkened to ~128 (0x80)
        let p0 = buffer.get_pixel(0, 0).unwrap();
        let r0 = (p0 >> 16) & 0xFF;
        assert!((r0 as i32 - 128).abs() <= 1, "Line 0 should be darkened");

        // Line 1: Untouched
        assert_eq!(
            buffer.get_pixel(0, 1).unwrap(),
            0xFFFF_FFFF,
            "Line 1 should be untouched"
        );

        // Line 2: Darkened
        let p2 = buffer.get_pixel(0, 2).unwrap();
        let r2 = (p2 >> 16) & 0xFF;
        assert!((r2 as i32 - 128).abs() <= 1, "Line 2 should be darkened");
    }

    #[test]
    fn test_chromatic_aberration() {
        let mut buffer = Framebuffer::new(3, 1).unwrap();
        // 0: Red, 1: Green, 2: Blue
        buffer.set_pixel(0, 0, 0xFFFF_0000);
        buffer.set_pixel(1, 0, 0xFF00_FF00);
        buffer.set_pixel(2, 0, 0xFF00_00FF);

        // Shift Red right by 1, Blue left by 1.
        // Pixel 1 (Center, Green) should get Red from Pixel 0 (Left) and Blue from Pixel 2 (Right).
        // Result Pixel 1 should be White (R from 0, G from 1, B from 2).

        // r_offset = -1 (fetch from left, meaning source x = current x + offset = 1 + (-1) = 0)
        // Wait, implementation: rx = (x + self.r_offset)
        // If I want to pull from LEFT, I need index - 1. So offset should be -1.

        let effect = ChromaticAberration::new(-1, 1);
        effect.apply(&mut buffer);

        let p1 = buffer.get_pixel(1, 0).unwrap();
        // R came from pixel 0 (Red 255)
        // G came from pixel 1 (Green 255)
        // B came from pixel 2 (Blue 255)
        assert_eq!(p1, 0xFFFF_FFFF);
    }
}

/// Inverts colors (creates a negative image).
pub struct Invert;

impl PostProcess for Invert {
    fn apply(&self, buffer: &mut Framebuffer) {
        let pixels = buffer.as_mut_slice();
        for pixel in pixels.iter_mut() {
            // Keep alpha (0xFF000000) intact, invert RGB
            // XOR with 0x00FFFFFF flips the lower 24 bits
            *pixel ^= 0x00FF_FFFF;
        }
    }
}

/// Adds retro CRT scanlines by darkening every Nth row.
pub struct Scanlines {
    /// The intensity of the darkening (0.0 to 1.0). 0.0 means no darkening.
    pub intensity: f32,
    /// The spacing of scanlines in pixels (e.g., 2 for every other line).
    pub spacing: u32,
}

impl Scanlines {
    #[must_use]
    pub const fn new(intensity: f32, spacing: u32) -> Self {
        Self { intensity, spacing }
    }
}

impl PostProcess for Scanlines {
    fn apply(&self, buffer: &mut Framebuffer) {
        let width = buffer.width();
        let height = buffer.height();

        // If spacing is 0 or 1, we skip or darken everything, let's just handle > 1
        if self.spacing <= 1 {
            return;
        }

        // Pre-calculate darken factor as fixed point 8.8
        // e.g. intensity 0.5 -> factor 128 -> pixel * 128 / 256 = pixel * 0.5
        let factor = ((1.0 - self.intensity) * 256.0) as u32;

        for y in 0..height {
            if y % self.spacing != 0 {
                continue;
            }

            for x in 0..width {
                let idx = (y * width + x) as usize;
                // Safe access because we iterate within bounds
                if let Some(pixel) = buffer.as_mut_slice().get_mut(idx) {
                    let r = (((*pixel >> 16) & 0xFF) * factor) >> 8;
                    let g = (((*pixel >> 8) & 0xFF) * factor) >> 8;
                    let b = ((*pixel & 0xFF) * factor) >> 8;
                    *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}

/// Simulates chromatic aberration by shifting RGB channels.
pub struct ChromaticAberration {
    /// Horizontal offset for Red channel
    pub r_offset: i32,
    /// Horizontal offset for Blue channel (Green stays center)
    pub b_offset: i32,
}

impl ChromaticAberration {
    #[must_use]
    pub const fn new(r_offset: i32, b_offset: i32) -> Self {
        Self { r_offset, b_offset }
    }
}

impl PostProcess for ChromaticAberration {
    fn apply(&self, buffer: &mut Framebuffer) {
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;

        // We need a copy of the source buffer to sample from shifted coordinates
        let original = buffer.as_slice().to_vec();

        let pixels = buffer.as_mut_slice();

        for y in 0..height {
            for x in 0..width {
                let center_idx = (y * width + x) as usize;

                // Green (Anchor) - sample from current position
                let g = (original[center_idx] >> 8) & 0xFF;

                // Red (Shifted)
                let rx = (x + self.r_offset).clamp(0, width - 1);
                let r_idx = (y * width + rx) as usize;
                let r = (original[r_idx] >> 16) & 0xFF;

                // Blue (Shifted)
                let bx = (x + self.b_offset).clamp(0, width - 1);
                let b_idx = (y * width + bx) as usize;
                let b = original[b_idx] & 0xFF;

                pixels[center_idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    }
}

/// A pipeline that chains multiple effects.
pub struct Pipeline {
    effects: Vec<Box<dyn PostProcess>>,
}

impl Pipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn add(mut self, effect: Box<dyn PostProcess>) -> Self {
        self.effects.push(effect);
        self
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcess for Pipeline {
    fn apply(&self, buffer: &mut Framebuffer) {
        for effect in &self.effects {
            effect.apply(buffer);
        }
    }
}
