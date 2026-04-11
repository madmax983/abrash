//! Pencil Sketch Post-Processing Filter
//!
//! A retro-style filter that simulates a pencil sketch by:
//! 1. Inverting the image.
//! 2. Applying a Gaussian or Box blur.
//! 3. Blending the blurred inverted image with the original image using Color Dodge.
//! 4. Converting the result to grayscale.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

use std::cell::RefCell;

thread_local! {
    static INVERTED_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static BLURRED_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static TEMP_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a pencil sketch effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `blur_radius` - The radius of the blur applied to the inverted image.
pub fn apply_pencil_sketch(fb: &mut Framebuffer, blur_radius: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let num_pixels = width * height;

    if num_pixels == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    INVERTED_BUFFER.with(|inv| {
        BLURRED_BUFFER.with(|blur| {
            TEMP_BUFFER.with(|temp| {
            let mut inv = inv.borrow_mut();
            let mut blur = blur.borrow_mut();
            let mut temp = temp.borrow_mut();

            inv.clear();
            inv.extend(pixels.iter().map(|&p| {
                let a = p & 0xFF000000;
                let r = 255 - ((p >> 16) & 0xFF);
                let g = 255 - ((p >> 8) & 0xFF);
                let b = 255 - (p & 0xFF);
                a | (r << 16) | (g << 8) | b
            }));

            blur.clear();
            blur.extend(std::iter::repeat(0).take(num_pixels));
            temp.clear();
            temp.extend(std::iter::repeat(0).take(num_pixels));

            // Box blur (horizontal pass)
            if blur_radius == 0 {
                temp.copy_from_slice(&inv);
                blur.copy_from_slice(&inv);
            } else {
                for y in 0..height {
                    for x in 0..width {
                        let mut r_sum = 0;
                        let mut g_sum = 0;
                        let mut b_sum = 0;
                        let mut count = 0;

                        let y_offset = y * width;
                        let min_x = x.saturating_sub(blur_radius as usize);
                        let max_x = std::cmp::min(x + blur_radius as usize, width - 1);

                        for k in min_x..=max_x {
                            let p = inv[y_offset + k];
                            r_sum += (p >> 16) & 0xFF;
                            g_sum += (p >> 8) & 0xFF;
                            b_sum += p & 0xFF;
                            count += 1;
                        }

                        if count > 0 {
                            let a = inv[y_offset + x] & 0xFF000000;
                            temp[y_offset + x] = a | ((r_sum / count) << 16) | ((g_sum / count) << 8) | (b_sum / count);
                        }
                    }
                }

                // Box blur (vertical pass)
                for y in 0..height {
                    let min_y = y.saturating_sub(blur_radius as usize);
                    let max_y = std::cmp::min(y + blur_radius as usize, height - 1);

                    for x in 0..width {
                        let mut r_sum = 0;
                        let mut g_sum = 0;
                        let mut b_sum = 0;
                        let mut count = 0;

                        for k in min_y..=max_y {
                            let p = temp[k * width + x];
                            r_sum += (p >> 16) & 0xFF;
                            g_sum += (p >> 8) & 0xFF;
                            b_sum += p & 0xFF;
                            count += 1;
                        }

                        if count > 0 {
                            let a = temp[y * width + x] & 0xFF000000;
                            blur[y * width + x] = a | ((r_sum / count) << 16) | ((g_sum / count) << 8) | (b_sum / count);
                        }
                    }
                }
            }

            // Color Dodge and Grayscale
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                let blur_slice: &[u32] = &blur;
                pixels.par_iter_mut().enumerate().for_each(|(i, p)| {
                    let top = blur_slice[i];
                    let bottom = *p;

                    let a = bottom & 0xFF000000;

                    let r_bot = (bottom >> 16) & 0xFF;
                    let g_bot = (bottom >> 8) & 0xFF;
                    let b_bot = bottom & 0xFF;

                    let r_top = (top >> 16) & 0xFF;
                    let g_top = (top >> 8) & 0xFF;
                    let b_top = top & 0xFF;

                    let color_dodge = |b: u32, t: u32| -> u32 {
                        if t == 255 {
                            255
                        } else {
                            let res = (b << 8) / (255 - t);
                            if res > 255 { 255 } else { res }
                        }
                    };

                    let r = color_dodge(r_bot, r_top);
                    let g = color_dodge(g_bot, g_top);
                    let b = color_dodge(b_bot, b_top);

                    let lum = pixel_luminance(a | (r << 16) | (g << 8) | b);
                    *p = a | ((lum as u32) << 16) | ((lum as u32) << 8) | (lum as u32);
                });
            }

            #[cfg(not(feature = "parallel"))]
            {
                for i in 0..num_pixels {
                    let top = blur[i];
                    let bottom = pixels[i];

                    let a = bottom & 0xFF000000;

                    let r_bot = (bottom >> 16) & 0xFF;
                    let g_bot = (bottom >> 8) & 0xFF;
                    let b_bot = bottom & 0xFF;

                    let r_top = (top >> 16) & 0xFF;
                    let g_top = (top >> 8) & 0xFF;
                    let b_top = top & 0xFF;

                    let color_dodge = |b: u32, t: u32| -> u32 {
                        if t == 255 {
                            255
                        } else {
                            let res = (b << 8) / (255 - t);
                            if res > 255 { 255 } else { res }
                        }
                    };

                    let r = color_dodge(r_bot, r_top);
                    let g = color_dodge(g_bot, g_top);
                    let b = color_dodge(b_bot, b_top);

                    let lum = pixel_luminance(a | (r << 16) | (g << 8) | b);
                    pixels[i] = a | ((lum as u32) << 16) | ((lum as u32) << 8) | (lum as u32);
                }
            }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pencil_sketch() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Original colors
        fb.set_pixel(0, 0, 0xFF000000); // Black
        fb.set_pixel(1, 0, 0xFFFFFFFF); // White
        fb.set_pixel(0, 1, 0xFFFF0000); // Red
        fb.set_pixel(1, 1, 0xFF00FF00); // Green

        apply_pencil_sketch(&mut fb, 1);

        // Check pixel (0, 0)
        let p = fb.get_pixel(0, 0).unwrap();
        // Since original is black (R=0, G=0, B=0), inverted is white (R=255, G=255, B=255).
        // Then blurred. Since it's a 2x2 image and we blur with radius 1, it will be somewhat white or gray.
        // Color dodge with bottom=0 and top > 0 will result in 0. So it stays black.
        // Wait, Color Dodge: `(b << 8) / (255 - t)`.
        // If bottom (original) is black (0), then `0 / anything` = 0.
        // So black stays black!
        assert_eq!(p, 0xFF000000);

        // White (255, 255, 255). Inverted is (0,0,0).
        // Bottom is 255. `255 << 8 / (255 - t)`.
        // If bottom is 255, it becomes white (255).
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFFFFFF);
    }
}
