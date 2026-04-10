//! Brickify Post-Processing Filter
//!
//! A retro-style filter that turns the framebuffer into a scene made of interlocking plastic bricks.
//! It pixelates the image by averaging colors within a block and then applies procedural
//! highlights and shadows to simulate a 3D stud on each block.

use crate::framebuffer::Framebuffer;

/// Applies a "brickify" (Lego-style) effect to the framebuffer.
///
/// Divides the framebuffer into blocks of `block_size` x `block_size`.
/// Each block is filled with the average color of its original pixels,
/// and then a procedural bevel and circular stud are drawn on top.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `block_size` - The size of the plastic bricks (must be >= 4 to see the stud).
pub fn apply_brickify(fb: &mut Framebuffer, block_size: u32) {
    if block_size < 4 {
        return;
    }

    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    let height = fb.height() as usize;
    if height == 0 {
        return;
    }

    let b_size = block_size as usize;

    // Bolt Performance Optimization:
    // To avoid allocating a `Vec` via `.to_vec()` on every frame, we use a single thread-local
    // buffer to store the copy of the framebuffer required by this filter. This avoids memory
    // fragmentation and reduces heap allocations.
    thread_local! {
        static ORIGINAL_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    ORIGINAL_PIXELS.with(|cell| {
        let mut src_pixels_vec = cell.borrow_mut();
        src_pixels_vec.clear();
        src_pixels_vec.extend_from_slice(fb.as_slice());
        let src_pixels = src_pixels_vec.as_slice();
        let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        let chunk_size = width * b_size;

        pixels
            .par_chunks_exact_mut(chunk_size)
            .enumerate()
            .for_each(|(chunk_idx, block_rows)| {
                let y_start = chunk_idx * b_size;
                let block_height = block_rows.len() / width;
                if block_height == 0 {
                    return;
                }

                for x in (0..width).step_by(b_size) {
                    let block_width = std::cmp::min(b_size, width - x);

                    // 1. Calculate Average Color
                    let avg_color = calculate_average_color(
                        &src_pixels,
                        width,
                        x,
                        y_start,
                        block_width,
                        block_height,
                    );

                    // 2. Draw the Brick (Bevel + Stud)
                    draw_brick(
                        block_rows,
                        width,
                        x,
                        block_width,
                        block_height,
                        b_size,
                        avg_color,
                    );
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in (0..height).step_by(b_size) {
            let block_height = std::cmp::min(b_size, height - y);
            let row_start = y * width;

            for x in (0..width).step_by(b_size) {
                let block_width = std::cmp::min(b_size, width - x);

                // 1. Calculate Average Color
                let avg_color =
                    calculate_average_color(&src_pixels, width, x, y, block_width, block_height);

                // 2. Draw the Brick (Bevel + Stud)
                // We pass the slice starting at `row_start` so local Y is 0 for this block
                let block_slice = &mut pixels[row_start..];
                draw_brick(
                    block_slice,
                    width,
                    x,
                    block_width,
                    block_height,
                    b_size,
                    avg_color,
                );
            }
        }
    }
    });
}

#[inline(always)]
fn calculate_average_color(
    src: &[u32],
    stride: usize,
    x_start: usize,
    y_start: usize,
    width: usize,
    height: usize,
) -> u32 {
    let mut sum_r: u32 = 0;
    let mut sum_g: u32 = 0;
    let mut sum_b: u32 = 0;
    let mut count: u32 = 0;

    for dy in 0..height {
        let row_idx = (y_start + dy) * stride;
        for dx in 0..width {
            let pixel = src[row_idx + x_start + dx];
            sum_r += (pixel >> 16) & 0xFF;
            sum_g += (pixel >> 8) & 0xFF;
            sum_b += pixel & 0xFF;
            count += 1;
        }
    }

    if count == 0 {
        return 0xFF00_0000;
    }

    let r = sum_r / count;
    let g = sum_g / count;
    let b = sum_b / count;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

#[inline(always)]
fn draw_brick(
    dst: &mut [u32],
    stride: usize,
    x_offset: usize,
    block_width: usize,
    block_height: usize,
    b_size: usize,
    base_color: u32,
) {
    let highlight = blend_color(base_color, 0xFFFF_FFFF, 64); // 25% white
    let shadow = blend_color(base_color, 0xFF00_0000, 64); // 25% black
    let deep_shadow = blend_color(base_color, 0xFF00_0000, 128); // 50% black

    // Center of the stud
    let cx = block_width as f32 / 2.0;
    let cy = block_height as f32 / 2.0;
    // Radius of the stud (leave some margin)
    let radius = (b_size as f32 * 0.35).max(1.0);
    let r2 = radius * radius;
    // Inner radius for the top flat part of the stud
    let inner_r2 = (radius * 0.7) * (radius * 0.7);

    for dy in 0..block_height {
        let row_idx = dy * stride + x_offset;
        for dx in 0..block_width {
            let mut color = base_color;

            // Block Bevels (Outer edges)
            if dy == 0 || dx == 0 {
                color = highlight; // Top and left edges catch light
            } else if dy == block_height - 1 || dx == block_width - 1 {
                color = deep_shadow; // Bottom and right edges in shadow
            } else if dy == 1 || dx == 1 {
                // Secondary highlight for slight rounding
                color = blend_color(base_color, 0xFFFF_FFFF, 32);
            } else if dy == block_height - 2 || dx == block_width - 2 {
                color = shadow;
            } else {
                // Inside the block, check for the stud
                let fx = dx as f32 - cx + 0.5;
                let fy = dy as f32 - cy + 0.5;
                let dist2 = fx * fx + fy * fy;

                if dist2 <= r2 {
                    if dist2 <= inner_r2 {
                        // Flat top of the stud
                        color = base_color;
                    } else {
                        // Slanted side of the stud
                        // Top-left side catches light, bottom-right side is in shadow
                        if fx + fy < 0.0 {
                            color = highlight;
                        } else {
                            color = shadow;
                        }
                    }
                }
            }

            dst[row_idx + dx] = color;
        }
    }
}

/// Fast integer blending between two colors.
/// `alpha` is 0-256, where 0 is 100% c1, 256 is 100% c2.
#[inline(always)]
const fn blend_color(c1: u32, c2: u32, alpha: u32) -> u32 {
    let inv_alpha = 256 - alpha;

    let r1 = (c1 >> 16) & 0xFF;
    let g1 = (c1 >> 8) & 0xFF;
    let b1 = c1 & 0xFF;

    let r2 = (c2 >> 16) & 0xFF;
    let g2 = (c2 >> 8) & 0xFF;
    let b2 = c2 & 0xFF;

    let r = (r1 * inv_alpha + r2 * alpha) >> 8;
    let g = (g1 * inv_alpha + g2 * alpha) >> 8;
    let b = (b1 * inv_alpha + b2 * alpha) >> 8;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_brickify_modifies_colors() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill with solid gray
        fb.clear(0xFF_808080);

        apply_brickify(&mut fb, 10);

        // Since it's a solid color, the average is the same.
        // However, the edges should be highlights/shadows.

        // Top-left should be a highlight (brighter than 0x80)
        let tl = fb.get_pixel(0, 0).unwrap();
        let tl_r = (tl >> 16) & 0xFF;
        assert!(tl_r > 0x80, "Top-left edge should be highlighted");

        // Bottom-right should be a shadow (darker than 0x80)
        let br = fb.get_pixel(9, 9).unwrap();
        let br_r = (br >> 16) & 0xFF;
        assert!(br_r < 0x80, "Bottom-right edge should be shadowed");

        // Center of the stud should be the base color
        let center = fb.get_pixel(5, 5).unwrap();
        assert_eq!(
            center, 0xFF_808080,
            "Center of stud should remain base color"
        );
    }
}
