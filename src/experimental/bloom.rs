//! Bloom Post-Processing Effect
//!
//! "Glow" effect for bright parts of the image.

use crate::framebuffer::Framebuffer;
use crate::light::{color_to_u32, u32_to_color};
use crate::math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct BloomParams {
    pub threshold: f32, // 0.0 to 1.0, brightness threshold
    pub blur_radius: usize,
    pub intensity: f32,
}

impl Default for BloomParams {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            blur_radius: 2,
            intensity: 1.0,
        }
    }
}

pub fn apply_bloom(fb: &mut Framebuffer, params: BloomParams) {
    // 1. Extract highlights
    let highlights = extract_highlights(fb, params.threshold);

    // 2. Blur highlights
    let blurred = box_blur(&highlights, params.blur_radius);

    // 3. Composite
    composite(fb, &blurred, params.intensity);
}

fn extract_highlights(fb: &Framebuffer, threshold: f32) -> Framebuffer {
    let mut out = Framebuffer::new(fb.width(), fb.height());
    let pixels = fb.as_slice();
    let out_pixels = out.as_mut_slice();

    for (i, &p) in pixels.iter().enumerate() {
        let color = u32_to_color(p);
        let luma = 0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z;
        if luma > threshold {
            out_pixels[i] = p;
        } else {
            out_pixels[i] = 0xFF000000; // Black
        }
    }
    out
}

fn box_blur(fb: &Framebuffer, radius: usize) -> Framebuffer {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let r = radius as i32;

    // Horizontal pass
    let mut temp = Framebuffer::new(fb.width(), fb.height());
    for y in 0..height {
        for x in 0..width {
            let mut sum_r = 0.0;
            let mut sum_g = 0.0;
            let mut sum_b = 0.0;
            let mut count = 0.0;

            for k in -r..=r {
                if let Some(p) = fb.get_pixel(x + k, y) {
                    let c = u32_to_color(p);
                    sum_r += c.x;
                    sum_g += c.y;
                    sum_b += c.z;
                    count += 1.0;
                }
            }

            if count > 0.0 {
                let avg = Vec3::new(sum_r / count, sum_g / count, sum_b / count);
                temp.set_pixel(x, y, color_to_u32(avg));
            }
        }
    }

    // Vertical pass
    let mut out = Framebuffer::new(fb.width(), fb.height());
    for x in 0..width {
        for y in 0..height {
            let mut sum_r = 0.0;
            let mut sum_g = 0.0;
            let mut sum_b = 0.0;
            let mut count = 0.0;

            for k in -r..=r {
                if let Some(p) = temp.get_pixel(x, y + k) {
                    let c = u32_to_color(p);
                    sum_r += c.x;
                    sum_g += c.y;
                    sum_b += c.z;
                    count += 1.0;
                }
            }

            if count > 0.0 {
                let avg = Vec3::new(sum_r / count, sum_g / count, sum_b / count);
                out.set_pixel(x, y, color_to_u32(avg));
            }
        }
    }

    out
}

fn composite(base: &mut Framebuffer, overlay: &Framebuffer, intensity: f32) {
    let base_pixels = base.as_mut_slice();
    let overlay_pixels = overlay.as_slice();

    for (b_pix, o_pix) in base_pixels.iter_mut().zip(overlay_pixels.iter()) {
        let b_color = u32_to_color(*b_pix);
        let o_color = u32_to_color(*o_pix);

        let final_color = b_color + o_color * intensity;
        // color_to_u32 handles clamping
        *b_pix = color_to_u32(final_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_highlights() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White (Luma ~1.0)
        fb.set_pixel(1, 1, 0xFF000000); // Black (Luma 0.0)

        let highlights = extract_highlights(&fb, 0.5);

        assert_eq!(highlights.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);
        assert_eq!(highlights.get_pixel(1, 1).unwrap(), 0xFF000000);
    }

    #[test]
    fn test_box_blur() {
        let mut fb = Framebuffer::new(3, 3);
        // Center pixel white
        fb.set_pixel(1, 1, 0xFFFFFFFF);

        let blurred = box_blur(&fb, 1);

        // Center should be dimmer than white
        let center = u32_to_color(blurred.get_pixel(1, 1).unwrap());
        assert!(center.x < 1.0);

        // Neighbor (0, 1) should have some color
        let neighbor = u32_to_color(blurred.get_pixel(0, 1).unwrap());
        assert!(neighbor.x > 0.0);
    }

    #[test]
    fn test_bloom_end_to_end() {
        let mut fb = Framebuffer::new(5, 5);
        fb.clear(0xFF000000);
        fb.set_pixel(2, 2, 0xFFFFFFFF);

        let params = BloomParams {
            threshold: 0.5,
            blur_radius: 1,
            intensity: 1.0,
        };

        apply_bloom(&mut fb, params);

        // Neighbor should now have color
        let neighbor = fb.get_pixel(2, 1).unwrap();
        assert_ne!(neighbor, 0xFF000000, "Neighbor should be glowing");
    }
}
