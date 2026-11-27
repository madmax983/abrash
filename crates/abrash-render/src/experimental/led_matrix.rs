//! LED Matrix Filter
//!
//! A post-processing effect that converts the image into an LED matrix display,
//! grouping pixels into cells and drawing a glowing circular LED for each cell
//! to simulate a jumbotron or pixel display.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use std::cell::RefCell;
thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Largest `cell_size` for which the per-row multiplier table lives on the stack.
const MAX_TABLE_CELL: usize = 64;

/// Configuration for the LED Matrix post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct LedMatrixConfig {
    /// The size of each LED cell in pixels.
    pub cell_size: usize,
    /// The radius of the glowing LED within the cell. Should be <= `cell_size` / 2.
    pub led_radius: f32,
    /// How much to darken the background (0.0 = black, 1.0 = original color).
    pub background_darken: f32,
    /// How much to darken the LED edge for an antialiased look (0.0 = sharp, 1.0 = soft).
    pub edge_softness: f32,
}

impl Default for LedMatrixConfig {
    fn default() -> Self {
        Self {
            cell_size: 10,
            led_radius: 4.0,
            background_darken: 0.1,
            edge_softness: 1.0,
        }
    }
}

/// Applies an LED Matrix effect to the framebuffer.
///
/// Converts the image into a pixelated LED style by sampling the center of each cell
/// and drawing a circular dot with the sampled color, leaving the rest dark.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the LED matrix effect.
pub fn apply_led_matrix(fb: &mut Framebuffer, config: &LedMatrixConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.cell_size <= 1 {
        return;
    }

    let cell_size = config.cell_size;
    let half_cell = cell_size / 2;
    let led_radius_sq = config.led_radius * config.led_radius;

    // Clone the source framebuffer because we are reading non-linearly
    // ⚡ Bolt: Eliminate per-frame heap allocation by using a thread-local static buffer.
    let mut source_pixels = SOURCE_PIXELS.with(RefCell::take);
    let size = width * height;
    if source_pixels.len() < size {
        source_pixels.resize(size, 0);
    }
    source_pixels[..size].copy_from_slice(fb.as_slice());

    let src_pixels = &source_pixels[..size]; // reference to local
    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = dest_pixels.chunks_exact_mut(width).enumerate();

    let bg_mult = (config.background_darken * 256.0).clamp(0.0, 256.0) as u32;

    // ⚡ Bolt: cell lookups, `dy²` and the LED/edge/background decision depend only on
    // the row and on `dx = x - center_x`, so compute one multiplier per `dx` per row and
    // walk each row in per-cell runs. A multiplier of 256 is the identity in
    // `darken_pixel`, so inside-LED pixels stay bit-identical.
    let table_offset = cell_size;
    iter.for_each(|(y, row)| {
        let cell_y = y / cell_size;
        let center_y = (cell_y * cell_size + half_cell).min(height - 1);
        let dy = y as f32 - center_y as f32;
        let dy_sq = dy * dy;
        let src_row = &src_pixels[center_y * width..(center_y + 1) * width];

        let mult_for = |dx: f32| -> u32 {
            let dist_sq = dx * dx + dy_sq;
            if dist_sq <= led_radius_sq {
                if config.edge_softness > 0.0 {
                    let dist = dist_sq.sqrt();
                    let edge_dist = config.led_radius - dist;
                    if edge_dist < config.edge_softness {
                        let factor = edge_dist / config.edge_softness;
                        return (factor * 256.0) as u32;
                    }
                }
                256
            } else {
                bg_mult
            }
        };

        let mut table = [0u32; 2 * MAX_TABLE_CELL + 1];
        let use_table = cell_size <= MAX_TABLE_CELL;
        if use_table {
            for (i, slot) in table[..=2 * cell_size].iter_mut().enumerate() {
                *slot = mult_for(i as f32 - table_offset as f32);
            }
        }

        let mut x0 = 0;
        while x0 < width {
            let x1 = (x0 + cell_size).min(width);
            let center_x = ((x0 / cell_size) * cell_size + half_cell).min(width - 1);
            let src_pixel = src_row[center_x];
            for (x, pixel) in row[x0..x1].iter_mut().enumerate().map(|(i, p)| (x0 + i, p)) {
                let mult = if use_table {
                    // x - center_x lies in [-cell_size, cell_size]
                    table[(x + table_offset) - center_x]
                } else {
                    mult_for(x as f32 - center_x as f32)
                };
                *pixel = if mult == 256 {
                    src_pixel
                } else {
                    darken_pixel(src_pixel, mult)
                };
            }
            x0 = x1;
        }
    });

    SOURCE_PIXELS.with(|buf| buf.replace(source_pixels));
}

#[inline(always)]
const fn darken_pixel(pixel: u32, darken_fixed: u32) -> u32 {
    let alpha = pixel & 0xFF00_0000;
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    let new_r = (r * darken_fixed) >> 8;
    let new_g = (g * darken_fixed) >> 8;
    let new_b = (b * darken_fixed) >> 8;

    alpha | (new_r << 16) | (new_g << 8) | new_b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_led_matrix_config_default() {
        let config = LedMatrixConfig::default();
        assert_eq!(config.cell_size, 10);
        assert!((config.led_radius - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_led_matrix() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Set all to white
        fb.clear(0xFF_FF_FF_FF);

        let config = LedMatrixConfig {
            cell_size: 10,
            led_radius: 4.0,
            background_darken: 0.0, // Pure black background
            edge_softness: 0.0,
        };
        apply_led_matrix(&mut fb, &config);

        // Center of the first cell (5,5) should be white
        let center_pixel = fb.get_pixel(5, 5).unwrap();
        assert_eq!(center_pixel, 0xFF_FF_FF_FF);

        // Corner of the first cell (0,0) should be black
        let corner_pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(corner_pixel, 0xFF_00_00_00);
    }

    /// The original per-pixel formulation, kept as a reference.
    fn reference(src: &[u32], width: usize, height: usize, config: &LedMatrixConfig) -> Vec<u32> {
        let cell_size = config.cell_size;
        let half_cell = cell_size / 2;
        let r_sq = config.led_radius * config.led_radius;
        let bg_mult = (config.background_darken * 256.0).clamp(0.0, 256.0) as u32;
        let mut out = vec![0; width * height];
        for y in 0..height {
            for x in 0..width {
                let cx = ((x / cell_size) * cell_size + half_cell).min(width - 1);
                let cy = ((y / cell_size) * cell_size + half_cell).min(height - 1);
                let sp = src[cy * width + cx];
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                let d2 = dx * dx + dy * dy;
                out[y * width + x] = if d2 <= r_sq {
                    if config.edge_softness > 0.0 {
                        let edge = config.led_radius - d2.sqrt();
                        if edge < config.edge_softness {
                            darken_pixel(sp, (edge / config.edge_softness * 256.0) as u32)
                        } else {
                            sp
                        }
                    } else {
                        sp
                    }
                } else {
                    darken_pixel(sp, bg_mult)
                };
            }
        }
        out
    }

    #[test]
    fn matches_reference_for_many_shapes() {
        for &(w, h) in &[(37usize, 23usize), (64, 64), (101, 70), (5, 5)] {
            let mut fb = Framebuffer::new(w as u32, h as u32).unwrap();
            for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
                *p = 0xFF00_0000 | (i as u32).wrapping_mul(2_654_435_761) >> 8;
            }
            let src = fb.as_slice().to_vec();
            for &cell in &[2usize, 3, 7, 10, 64, 70] {
                for &(rad, soft, bg) in &[
                    (3.5f32, 1.0f32, 0.1f32),
                    (4.0, 0.0, 0.0),
                    (2.0, 3.0, 0.5),
                    (100.0, 1.0, 1.0),
                ] {
                    let config = LedMatrixConfig {
                        cell_size: cell,
                        led_radius: rad,
                        background_darken: bg,
                        edge_softness: soft,
                    };
                    let mut test_fb = Framebuffer::new(w as u32, h as u32).unwrap();
                    test_fb.as_mut_slice().copy_from_slice(&src);
                    apply_led_matrix(&mut test_fb, &config);
                    assert_eq!(
                        test_fb.as_slice(),
                        &reference(&src, w, h, &config)[..],
                        "{w}x{h} cell {cell} {config:?}"
                    );
                }
            }
        }
    }
}
