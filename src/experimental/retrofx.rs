//! `RetroFX`: Post-processing pipeline for retro visual effects.
//!
//! Provides a composable pipeline to apply screen-space effects like grayscale,
//! scanlines, and chromatic aberration.

use crate::framebuffer::Framebuffer;

/// Trait for post-processing effects.
pub trait PostEffect {
    /// Apply the effect to the framebuffer in-place.
    fn apply(&self, fb: &mut Framebuffer);
}

/// A pipeline that chains multiple post-processing effects.
#[derive(Default)]
pub struct RetroPipeline {
    effects: Vec<Box<dyn PostEffect>>,
}

impl RetroPipeline {
    /// Creates a new, empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self { effects: Vec::new() }
    }

    /// Adds an effect to the pipeline.
    #[must_use]
    pub fn with_effect<E: PostEffect + 'static>(mut self, effect: E) -> Self {
        self.effects.push(Box::new(effect));
        self
    }

    /// Applies all effects in order to the framebuffer.
    pub fn apply(&self, fb: &mut Framebuffer) {
        for effect in &self.effects {
            effect.apply(fb);
        }
    }
}

/// helper to unpack u32 to (r, g, b)
#[inline(always)]
const fn unpack_color(c: u32) -> (u8, u8, u8) {
    let r = ((c >> 16) & 0xFF) as u8;
    let g = ((c >> 8) & 0xFF) as u8;
    let b = (c & 0xFF) as u8;
    (r, g, b)
}

/// helper to pack (r, g, b) to u32
#[inline(always)]
fn pack_color(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Grayscale effect using Luma coefficients.
pub struct Grayscale;

impl PostEffect for Grayscale {
    fn apply(&self, fb: &mut Framebuffer) {
        for pixel in fb.as_mut_slice() {
            let (r, g, b) = unpack_color(*pixel);
            // Luma = 0.299R + 0.587G + 0.114B
            let luma = ((f32::from(r) * 0.299) + (f32::from(g) * 0.587) + (f32::from(b) * 0.114)) as u8;
            *pixel = pack_color(luma, luma, luma);
        }
    }
}

/// Invert colors effect.
pub struct Invert;

impl PostEffect for Invert {
    fn apply(&self, fb: &mut Framebuffer) {
        for pixel in fb.as_mut_slice() {
            let (r, g, b) = unpack_color(*pixel);
            *pixel = pack_color(255 - r, 255 - g, 255 - b);
        }
    }
}

/// Scanlines effect: darkens every Nth line.
pub struct Scanlines {
    pub intensity: f32,
    pub stride: usize,
}

impl Scanlines {
    #[must_use]
    pub const fn new(intensity: f32, stride: usize) -> Self {
        Self { intensity, stride }
    }
}

impl Default for Scanlines {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            stride: 2,
        }
    }
}

impl PostEffect for Scanlines {
    fn apply(&self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let pixels = fb.as_mut_slice();

        for y in 0..height {
            if y % self.stride == 0 {
                let start = y * width;
                let end = start + width;
                for pixel in &mut pixels[start..end] {
                    let (r, g, b) = unpack_color(*pixel);
                    let r = (f32::from(r) * (1.0 - self.intensity)) as u8;
                    let g = (f32::from(g) * (1.0 - self.intensity)) as u8;
                    let b = (f32::from(b) * (1.0 - self.intensity)) as u8;
                    *pixel = pack_color(r, g, b);
                }
            }
        }
    }
}

/// Chromatic Aberration: Shifts Red channel left and Blue channel right.
pub struct ChromaticAberration {
    pub offset: i32,
}

impl ChromaticAberration {
    #[must_use]
    pub const fn new(offset: i32) -> Self {
        Self { offset }
    }
}

impl PostEffect for ChromaticAberration {
    fn apply(&self, fb: &mut Framebuffer) {
        // We need a copy of the buffer to read from while writing
        // This is expensive but necessary for neighborhood operations without artifacts
        let source_pixels = fb.as_slice().to_vec();
        let width = fb.width() as i32;
        let height = fb.height() as i32;

        for y in 0..height {
            for x in 0..width {
                // Green stays put
                let idx = (y * width + x) as usize;
                let (_, g, _) = unpack_color(source_pixels[idx]);

                // Red shifted left
                let r_x = (x - self.offset).clamp(0, width - 1);
                let r_idx = (y * width + r_x) as usize;
                let (r, _, _) = unpack_color(source_pixels[r_idx]);

                // Blue shifted right
                let b_x = (x + self.offset).clamp(0, width - 1);
                let b_idx = (y * width + b_x) as usize;
                let (_, _, b) = unpack_color(source_pixels[b_idx]);

                // Write back
                fb.set_pixel(x, y, pack_color(r, g, b));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grayscale() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red
        let pipeline = RetroPipeline::new().with_effect(Grayscale);
        pipeline.apply(&mut fb);

        let pixel = fb.get_pixel(0, 0).unwrap();
        let (r, g, b) = unpack_color(pixel);
        assert_eq!(r, g);
        assert_eq!(g, b);
        // Red (255, 0, 0) -> Luma ~76
        assert!((i32::from(r) - 76).abs() <= 1);
    }

    #[test]
    fn test_invert() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        let pipeline = RetroPipeline::new().with_effect(Invert);
        pipeline.apply(&mut fb);

        let pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(pixel, 0xFFFF_FFFF); // White
    }

    #[test]
    fn test_scanlines() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFFFF_FFFF); // White
        let pipeline = RetroPipeline::new().with_effect(Scanlines::new(1.0, 2)); // Black out every 2nd line
        pipeline.apply(&mut fb);

        // Line 0 should be black
        let p0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p0 & 0x00FF_FFFF, 0);

        // Line 1 should be white
        let p1 = fb.get_pixel(0, 1).unwrap();
        assert_eq!(p1, 0xFFFF_FFFF);
    }

    #[test]
    fn test_chromatic_aberration() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        // Left: Red, Center: Green, Right: Blue
        fb.set_pixel(0, 0, 0xFFFF_0000);
        fb.set_pixel(1, 0, 0xFF00_FF00);
        fb.set_pixel(2, 0, 0xFF00_00FF);

        // Shift by 1
        let pipeline = RetroPipeline::new().with_effect(ChromaticAberration::new(1));
        pipeline.apply(&mut fb);

        // Center pixel (1, 0):
        // Red comes from left (0, 0) -> Red
        // Green stays (1, 0) -> Green
        // Blue comes from right (2, 0) -> Blue
        // So center should be White (R+G+B)

        let p_center = fb.get_pixel(1, 0).unwrap();
        assert_eq!(p_center, 0xFFFF_FFFF);
    }
}
