//! Fisheye Lens Filter.
//!
//! A retro post-processing effect that distorts the image radially to simulate a
//! fisheye lens, creating a bulging or barrel-like distortion.

use crate::framebuffer::Framebuffer;

/// Configuration for the Fisheye Lens effect.
#[derive(Debug, Clone, Copy)]
pub struct FisheyeConfig {
    /// The center X coordinate of the lens (0.0 to 1.0).
    pub center_x: f32,
    /// The center Y coordinate of the lens (0.0 to 1.0).
    pub center_y: f32,
    /// The radius of the lens distortion.
    pub radius: f32,
    /// The strength of the distortion (how bulging it is).
    pub strength: f32,
}

impl Default for FisheyeConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 400.0,
            strength: 1.5,
        }
    }
}

use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Fisheye lens distortion to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the effect.
pub fn apply_fisheye(fb: &mut Framebuffer, config: &FisheyeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.radius <= 0.0 {
        return;
    }

    let mut source_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);

    let fb_slice = fb.as_slice();
    if source_pixels.len() != fb_slice.len() {
        source_pixels.resize(fb_slice.len(), 0);
    }
    source_pixels.copy_from_slice(fb_slice);

    let dest = fb.as_mut_slice();
    let src: &[u32] = &source_pixels;

    let center_x_px = config.center_x * width as f32;
    let center_y_px = config.center_y * height as f32;
    let radius_sq = config.radius * config.radius;

    #[cfg(feature = "parallel")]
    let row_iter = dest.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        let dy = y_f32 - center_y_px;
        let dy_sq = dy * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let x_f32 = x as f32;
            let dx = x_f32 - center_x_px;
            let dist_sq = dx * dx + dy_sq;

            if dist_sq < radius_sq {
                let distance = dist_sq.sqrt();
                let distortion =
                    distance.powf(config.strength) / (config.radius.powf(config.strength - 1.0));

                let scale = if distance > 0.0 {
                    distortion / distance
                } else {
                    0.0
                };

                let src_x = (center_x_px + dx * scale) as i32;
                let src_y = (center_y_px + dy * scale) as i32;

                let src_x = src_x.clamp(0, width as i32 - 1) as usize;
                let src_y = src_y.clamp(0, height as i32 - 1) as usize;

                *pixel = src[src_y * width + src_x];
            }
        }
    });

    SOURCE_PIXELS.with(|source_pixels_cell| source_pixels_cell.replace(source_pixels));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_fisheye() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        for y in 0..height {
            for x in 0..width {
                fb.set_pixel(x as i32, y as i32, 0xFF_FF_00_00);
            }
        }

        let config = FisheyeConfig {
            center_x: 0.5,
            center_y: 0.5,
            radius: 50.0,
            strength: 1.5,
        };

        apply_fisheye(&mut fb, &config);

        // Since we filled it all with red, the output should still be all red.
        // We're just testing it doesn't crash or go out of bounds.
        for i in 0..width * height {
            assert_eq!(fb.as_slice()[i as usize], 0xFF_FF_00_00);
        }
    }
}
