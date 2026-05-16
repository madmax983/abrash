//! CRT Monitor Post-Processing Filter
//!
//! Simulates the barrel distortion of a classic Cathode Ray Tube monitor.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

/// Applies a CRT monitor barrel distortion effect to the framebuffer.
///
/// Pixels are mapped using a radial distortion function to curve the image
/// away from the center, mimicking the curvature of a physical CRT screen.
///
/// # Performance
///
/// Optimization: Uses a `thread_local!` buffer instead of allocating a new
/// `Vec` every frame. This eliminates a costly dynamic heap allocation
/// per frame, significantly improving performance on the hot path while
/// preventing in-place overwrite artifacts.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `distortion` - The strength of the barrel distortion (e.g., 0.1 to 0.3).
struct CrtCache {
    nx2: Vec<f32>,
    base_x: Vec<f32>,
    dist_x: Vec<f32>,
    width: usize,
    distortion: f32,
}

pub fn apply_crt(fb: &mut Framebuffer, distortion: f32) {
    if distortion <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    thread_local! {
        static CRT_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        static CACHE: RefCell<CrtCache> = RefCell::new(CrtCache {
            nx2: Vec::new(),
            base_x: Vec::new(),
            dist_x: Vec::new(),
            width: 0,
            distortion: 0.0,
        });
    }

    CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();

        // Rebuild cache if resolution or distortion parameters changed
        if cache.width != width || (cache.distortion - distortion).abs() > f32::EPSILON {
            cache.width = width;
            cache.distortion = distortion;
            cache.nx2.resize(width, 0.0);
            cache.base_x.resize(width, 0.0);
            cache.dist_x.resize(width, 0.0);

            for x in 0..width {
                let nx = (x as f32 - cx) / cx;
                let nx2 = nx * nx;
                cache.nx2[x] = nx2;
                cache.dist_x[x] = nx * cx * distortion;
                cache.base_x[x] = nx * cx + nx * cx * distortion * nx2 + cx;
            }
        }

        let nx2_cache = &cache.nx2[..width];
        let base_x_cache = &cache.base_x[..width];
        let dist_x_cache = &cache.dist_x[..width];

        CRT_BUFFER.with(|buf| {
            let mut new_pixels_vec = buf.borrow_mut();
            let size = width * height;
            if new_pixels_vec.len() < size {
                new_pixels_vec.resize(size, 0xFF00_0000);
            }
            let new_pixels = &mut new_pixels_vec[..size];
            new_pixels.fill(0xFF00_0000);

            let pixels = fb.as_slice();
            let width_i32 = width as i32;
            let height_i32 = height as i32;

            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                new_pixels
                    .par_chunks_exact_mut(width)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let ny = (y as f32 - cy) / cy;
                        let ny2 = ny * ny;

                        let base_y = ny * cy + ny * cy * distortion * ny2 + cy;
                        let dist_y = ny * cy * distortion;

                        for (x, pixel) in row.iter_mut().enumerate().take(width) {
                            let src_x = (base_x_cache[x] + dist_x_cache[x] * ny2) as i32;
                            let src_y = (base_y + dist_y * nx2_cache[x]) as i32;

                            if src_x >= 0 && src_x < width_i32 && src_y >= 0 && src_y < height_i32 {
                                let src_idx = (src_y as usize) * width + (src_x as usize);
                                *pixel = pixels[src_idx];
                            }
                        }
                    });
            }

            #[cfg(not(feature = "parallel"))]
            {
                for y in 0..height {
                    let ny = (y as f32 - cy) / cy;
                    let ny2 = ny * ny;

                    let base_y = ny * cy + ny * cy * distortion * ny2 + cy;
                    let dist_y = ny * cy * distortion;

                    let row_offset = y * width;

                    for x in 0..width {
                        let src_x = (base_x_cache[x] + dist_x_cache[x] * ny2) as i32;
                        let src_y = (base_y + dist_y * nx2_cache[x]) as i32;

                        if src_x >= 0 && src_x < width_i32 && src_y >= 0 && src_y < height_i32 {
                            let src_idx = (src_y as usize) * width + (src_x as usize);
                            new_pixels[row_offset + x] = pixels[src_idx];
                        }
                    }
                }
            }

            fb.as_mut_slice().copy_from_slice(new_pixels);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_crt_determinism() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Set all pixels to some color
        for i in 0..9 {
            fb.as_mut_slice()[i] = 0xFF_FFFFFF; // White
        }

        // Apply distortion
        apply_crt(&mut fb, 0.5);

        let expected = [
            0xFF_000000,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
            0xFF_FFFFFF,
        ];

        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(
                    fb.get_pixel(x as i32, y as i32).unwrap(),
                    expected[y * 3 + x as usize],
                    "pixel at {}, {}",
                    x,
                    y
                );
            }
        }
    }
}
