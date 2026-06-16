//! Droste Effect Post-Processing Filter
//!
//! A recursive picture-in-picture effect.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Droste effect.
#[derive(Debug, Clone, Copy)]
pub struct DrosteConfig {
    /// Number of recursive layers (iterations).
    pub iterations: u32,
    /// Scale factor for each recursive step (e.g., 0.5 means inner picture is half size).
    pub scale: f32,
    /// Offset of the inner picture relative to the center, normalized (e.g., (0.0, 0.0) is centered).
    pub offset_x: f32,
    /// Vertical offset in the spiral.
    pub offset_y: f32,
}

impl Default for DrosteConfig {
    fn default() -> Self {
        Self {
            iterations: 4,
            scale: 0.5,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Applies the Droste effect to the framebuffer.
///
/// This effect recursively shrinks and overlays the image onto itself.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the droste effect.
/// Apply a Droste effect (recursive picture-in-picture) to the framebuffer.
///
/// Scales and maps coordinates logarithmically to repeat the image inside itself.
pub fn apply_droste(fb: &mut Framebuffer, config: &DrosteConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.iterations == 0 {
        return;
    }

    let len = width * height;

    // Cache the original framebuffer to prevent read-after-write aliasing
    SOURCE_BUFFER.with(|buf| {
        let mut b = buf.borrow_mut();
        if b.len() < len {
            b.resize(len, 0);
        }
        b[..len].copy_from_slice(fb.as_slice());
    });

    let dest_pixels = fb.as_mut_slice();

    let cx = width as f32 * 0.5 + config.offset_x * width as f32;
    let cy = height as f32 * 0.5 + config.offset_y * height as f32;

    SOURCE_BUFFER.with(|buf| {
        let src_pixels = buf.borrow();
        let src_pixels_slice: &[u32] = &src_pixels;

        let process_row = |y: usize, row: &mut [u32]| {
            let dy = y as f32 - cy;
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - cx;

                let mut sample_x = dx;
                let mut sample_y = dy;
                let mut sampled_color = src_pixels_slice[y * width + x];

                // Recursively unscale the coordinates to sample from the inner images
                for _ in 0..config.iterations {
                    let unscaled_x = sample_x / config.scale;
                    let unscaled_y = sample_y / config.scale;

                    let px = (unscaled_x + cx).round() as i32;
                    let py = (unscaled_y + cy).round() as i32;

                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        // The point falls inside the bounds of the original image when unscaled,
                        // which means in the final image it should display the inner version.
                        sample_x = unscaled_x;
                        sample_y = unscaled_y;
                        sampled_color = src_pixels_slice[(py as usize) * width + (px as usize)];
                    } else {
                        // We hit the boundary of an inner image, so we keep the current sampled_color.
                        break;
                    }
                }

                *pixel = sampled_color;
            }
        };

        #[cfg(feature = "parallel")]
        {
            dest_pixels
                .par_chunks_mut(width)
                .enumerate()
                .for_each(|(y, row)| process_row(y, row));
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_mut(width)
                .enumerate()
                .for_each(|(y, row)| process_row(y, row));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_droste() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000);
        // Draw a white border
        for x in 0..100 {
            fb.set_pixel(x, 0, 0xFFFF_FFFF);
            fb.set_pixel(x, 99, 0xFFFF_FFFF);
        }
        for y in 0..100 {
            fb.set_pixel(0, y, 0xFFFF_FFFF);
            fb.set_pixel(99, y, 0xFFFF_FFFF);
        }

        let config = DrosteConfig {
            iterations: 2,
            scale: 0.5,
            offset_x: 0.0,
            offset_y: 0.0,
        };

        apply_droste(&mut fb, &config);

        // Expect the inner border to exist at scaled coordinates
        // At scale 0.5, centered, the inner picture starts at (25, 25) and ends at (74, 74)
        assert_eq!(fb.get_pixel(25, 25).unwrap(), 0xFFFF_FFFF);
    }
}
