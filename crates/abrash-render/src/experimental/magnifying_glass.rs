//! Magnifying Glass Filter
//!
//! A post-processing effect that simulates a magnifying glass moving over the screen.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the magnifying glass effect.
#[derive(Debug, Clone, Copy)]
pub struct MagnifyingGlassConfig {
    /// The X coordinate of the center of the lens.
    pub center_x: i32,
    /// The Y coordinate of the center of the lens.
    pub center_y: i32,
    /// The radius of the lens in pixels.
    pub radius: i32,
    /// The magnification factor (e.g., 2.0 for 2x zoom).
    pub magnification: f32,
    /// The thickness of the lens border/rim in pixels.
    pub border_thickness: i32,
    /// The color of the lens border (0xAARRGGBB).
    pub border_color: u32,
}

impl Default for MagnifyingGlassConfig {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 0,
            radius: 100,
            magnification: 2.0,
            border_thickness: 3,
            border_color: 0xFF22_2222, // Dark gray
        }
    }
}

/// Applies a magnifying glass effect to the framebuffer.
///
/// Pixels within the lens radius are magnified by pulling from the source
/// closer to the lens center.
pub fn apply_magnifying_glass(fb: &mut Framebuffer, config: &MagnifyingGlassConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.radius <= 0 {
        return;
    }

    // We must read from the original pixels while writing to the buffer.
    SOURCE_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        let cx = config.center_x;
        let cy = config.center_y;
        let r2 = config.radius * config.radius;
        let border_inner_r2 = (config.radius - config.border_thickness).max(0).pow(2);

        let inv_mag = if config.magnification > 0.0 {
            1.0 / config.magnification
        } else {
            1.0
        };

        // Bounding box of the lens
        let start_y = (cy - config.radius).max(0) as usize;
        let end_y = (cy + config.radius).clamp(0, height as i32 - 1) as usize;

        // We only need to iterate over the bounding box of the lens,
        // because pixels outside it are unmodified.
        // dest_pixels contains the same data as src_pixels initially.

        for y in start_y..=end_y {
            let row_offset = y * width;
            let start_x = (cx - config.radius).max(0) as usize;
            let end_x = (cx + config.radius).clamp(0, width as i32 - 1) as usize;
            for x in start_x..=end_x {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                let dist2 = dx * dx + dy * dy;

                if dist2 <= r2 {
                    if dist2 >= border_inner_r2 {
                        // We are in the border
                        dest_pixels[row_offset + x] = config.border_color;
                    } else {
                        // We are inside the lens, magnify
                        // The source pixel coordinate
                        // dx_src = dx / mag, dy_src = dy / mag
                        let src_x = cx + (dx as f32 * inv_mag) as i32;
                        let src_y = cy + (dy as f32 * inv_mag) as i32;

                        let clamped_x = src_x.clamp(0, width as i32 - 1) as usize;
                        let clamped_y = src_y.clamp(0, height as i32 - 1) as usize;

                        dest_pixels[row_offset + x] = src_pixels[clamped_y * width + clamped_x];
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magnifying_glass_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill with black
        fb.clear(0xFF00_0000);

        // Draw a red pixel at the center (5, 5)
        fb.set_pixel(5, 5, 0xFFFF_0000);

        // Apply a magnifying glass over the center with 2x zoom
        let config = MagnifyingGlassConfig {
            center_x: 5,
            center_y: 5,
            radius: 3,
            magnification: 2.0,
            border_thickness: 0,
            border_color: 0,
        };

        apply_magnifying_glass(&mut fb, &config);

        // Because of the 2x magnification, the pixel at (5, 5) remains red
        assert_eq!(fb.get_pixel(5, 5), Some(0xFFFF_0000));

        // The pixel at (6, 5) would sample from dx=1, src_dx=0.5 -> 0, so src_x = 5.
        // This means the red pixel is magnified!
        assert_eq!(fb.get_pixel(6, 5), Some(0xFFFF_0000));
        assert_eq!(fb.get_pixel(4, 5), Some(0xFFFF_0000));
        assert_eq!(fb.get_pixel(5, 6), Some(0xFFFF_0000));
        assert_eq!(fb.get_pixel(5, 4), Some(0xFFFF_0000));
    }
}
