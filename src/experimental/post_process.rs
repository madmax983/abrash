//! Post-Processing Effects Pipeline
//!
//! Provides a framework for applying image effects to the framebuffer.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Trait for applying post-processing effects.
pub trait PostProcess {
    /// Applies the effect to the framebuffer.
    fn apply(&self, fb: &mut Framebuffer, zb: &ZBuffer);
}

/// Converts the image to grayscale using luminance weights.
pub struct Grayscale;

impl PostProcess for Grayscale {
    fn apply(&self, fb: &mut Framebuffer, _zb: &ZBuffer) {
        for pixel in fb.as_mut_slice() {
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;
            // BT.601 luminance coefficients
            let gray = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) as u32;
            *pixel = 0xFF00_0000 | (gray << 16) | (gray << 8) | gray;
        }
    }
}

/// Inverts the colors of the image.
pub struct Invert;

impl PostProcess for Invert {
    fn apply(&self, fb: &mut Framebuffer, _zb: &ZBuffer) {
        for pixel in fb.as_mut_slice() {
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;
            *pixel = 0xFF00_0000 | ((255 - r) << 16) | ((255 - g) << 8) | (255 - b);
        }
    }
}

/// Applies a classic sepia tone.
pub struct Sepia;

impl PostProcess for Sepia {
    fn apply(&self, fb: &mut Framebuffer, _zb: &ZBuffer) {
        for pixel in fb.as_mut_slice() {
            let r = ((*pixel >> 16) & 0xFF) as f32;
            let g = ((*pixel >> 8) & 0xFF) as f32;
            let b = (*pixel & 0xFF) as f32;

            let tr = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u32;
            let tg = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u32;
            let tb = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u32;

            *pixel = 0xFF00_0000 | (tr << 16) | (tg << 8) | tb;
        }
    }
}

/// Blends pixels with a fog color based on depth.
pub struct DepthFog {
    pub color: u32,
    pub near: f32,
    pub far: f32,
}

impl PostProcess for DepthFog {
    fn apply(&self, fb: &mut Framebuffer, zb: &ZBuffer) {
        let pixels = fb.as_mut_slice();
        let depths = zb.as_slice();

        let fog_r = ((self.color >> 16) & 0xFF) as f32;
        let fog_g = ((self.color >> 8) & 0xFF) as f32;
        let fog_b = (self.color & 0xFF) as f32;

        for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
            // Skip background/skybox (infinite depth) if desired,
            // or fog it completely? Usually infinite depth = max fog.
            // But if depth buffer is init to infinity, we might want to keep it clear or fully fogged.
            // Let's assume infinity is fully fogged.
            let factor = if depth == f32::INFINITY {
                1.0
            } else {
                ((depth - self.near) / (self.far - self.near)).clamp(0.0, 1.0)
            };

            let r = ((*pixel >> 16) & 0xFF) as f32;
            let g = ((*pixel >> 8) & 0xFF) as f32;
            let b = (*pixel & 0xFF) as f32;

            let out_r = r + (fog_r - r) * factor;
            let out_g = g + (fog_g - g) * factor;
            let out_b = b + (fog_b - b) * factor;

            *pixel = 0xFF00_0000 | ((out_r as u32) << 16) | ((out_g as u32) << 8) | (out_b as u32);
        }
    }
}

/// Simulates chromatic aberration by shifting color channels.
pub struct ChromaticAberration {
    /// Intensity of the shift (0.0 to 1.0 recommended)
    pub intensity: f32,
}

impl PostProcess for ChromaticAberration {
    fn apply(&self, fb: &mut Framebuffer, _zb: &ZBuffer) {
        let width = fb.width() as i32;
        let height = fb.height() as i32;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        // Clone source buffer for reading
        let src_pixels = fb.as_slice().to_vec();

        let get_pixel = |x: i32, y: i32| -> u32 {
            if x < 0 || x >= width || y < 0 || y >= height {
                0 // Black border
            } else {
                src_pixels[(y * width + x) as usize]
            }
        };

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;

                // Shift Red channel outward
                let r_off_x = (x as f32 - dx * self.intensity) as i32;
                let r_off_y = (y as f32 - dy * self.intensity) as i32;

                // Shift Blue channel inward
                let b_off_x = (x as f32 + dx * self.intensity) as i32;
                let b_off_y = (y as f32 + dy * self.intensity) as i32;

                let p_r = get_pixel(r_off_x, r_off_y);
                let p_g = get_pixel(x, y); // Green stays
                let p_b = get_pixel(b_off_x, b_off_y);

                let r = (p_r >> 16) & 0xFF;
                let g = (p_g >> 8) & 0xFF;
                let b = p_b & 0xFF;

                fb.set_pixel(x, y, 0xFF00_0000 | (r << 16) | (g << 8) | b);
            }
        }
    }
}

/// Adds CRT scanlines and vignette.
pub struct CRT {
    pub scanline_intensity: f32,
    pub vignette_intensity: f32,
}

impl PostProcess for CRT {
    fn apply(&self, fb: &mut Framebuffer, _zb: &ZBuffer) {
        let width_f = fb.width() as f32;
        let height_f = fb.height() as f32;
        let cx = width_f / 2.0;
        let cy = height_f / 2.0;
        let max_dist = cx.hypot(cy);

        // Optimize: Pre-calculate max_dist reciprocal
        let inv_max_dist = if max_dist > 0.0 { 1.0 / max_dist } else { 0.0 };

        let width = fb.width();
        let pixels = fb.as_mut_slice();

        for (i, pixel) in pixels.iter_mut().enumerate() {
            let x = (i as u32 % width) as i32;
            let y = (i as u32 / width) as i32;

            let scanline_factor = if y % 2 == 0 {
                1.0
            } else {
                1.0 - self.scanline_intensity
            };

            let r = ((*pixel >> 16) & 0xFF) as f32;
            let g = ((*pixel >> 8) & 0xFF) as f32;
            let b = (*pixel & 0xFF) as f32;

            // Vignette
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            let vig = 1.0 - (dist * inv_max_dist * self.vignette_intensity).min(1.0);

            let factor = scanline_factor * vig;

            let out_r = (r * factor) as u32;
            let out_g = (g * factor) as u32;
            let out_b = (b * factor) as u32;

            *pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_effect() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let zb = ZBuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red
        let effect = Grayscale;
        effect.apply(&mut fb, &zb);
        let pixel = fb.get_pixel(0, 0).unwrap();
        let gray = (pixel >> 16) & 0xFF;
        // Red contribution is ~0.299 * 255 = 76.245
        assert!((75..=77).contains(&gray), "Expected gray ~76, got {}", gray);
    }

    #[test]
    fn invert_effect() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let zb = ZBuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        let effect = Invert;
        effect.apply(&mut fb, &zb);
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF);
    }

    #[test]
    fn sepia_effect() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let zb = ZBuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF80_8080); // Gray 128
        let effect = Sepia;
        effect.apply(&mut fb, &zb);
        let pixel = fb.get_pixel(0, 0).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;
        // R > G > B for sepia
        assert!(r > g);
        assert!(g > b);
    }

    #[test]
    fn depth_fog_effect() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        let mut zb = ZBuffer::new(2, 1).unwrap();

        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        zb.test_and_set(0, 0, 10.0);

        fb.set_pixel(1, 0, 0xFF00_0000); // Black
        zb.test_and_set(1, 0, 20.0);

        let effect = DepthFog {
            color: 0xFFFF_FFFF, // White Fog
            near: 10.0,
            far: 20.0,
        };

        effect.apply(&mut fb, &zb);

        // Pixel 0: depth 10 = near. Factor 0. Color stays black.
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF00_0000);

        // Pixel 1: depth 20 = far. Factor 1. Color becomes white.
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFF_FFFF);
    }

    #[test]
    fn crt_scanlines() {
        let mut fb = Framebuffer::new(1, 2).unwrap(); // 2 lines
        let zb = ZBuffer::new(1, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_FFFF);
        fb.set_pixel(0, 1, 0xFFFF_FFFF);

        let effect = CRT {
            scanline_intensity: 0.5,
            vignette_intensity: 0.0,
        };

        effect.apply(&mut fb, &zb);

        let line0 = fb.get_pixel(0, 0).unwrap() & 0xFF;
        let line1 = fb.get_pixel(0, 1).unwrap() & 0xFF;

        assert_eq!(line0, 255); // Even line unchanged
        assert!(line1 < 255); // Odd line darkened
    }
}
