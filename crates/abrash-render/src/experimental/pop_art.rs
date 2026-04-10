//! Pop Art Post-Processing Filter
//!
//! A retro-style filter that divides the framebuffer into four quadrants,
//! scales the original image down into each quadrant, and applies a high-contrast
//! two-color thresholding effect to simulate Andy Warhol's silk-screen pop art.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Pop Art filter.
#[derive(Debug, Clone)]
pub struct PopArtConfig {
    /// Luminance threshold (0.0 to 1.0). Pixels darker than this use color1, lighter use color2.
    pub threshold: f32,
    /// Palette for the top-left quadrant (`dark_color`, `light_color`).
    pub palette_tl: (u32, u32),
    /// Palette for the top-right quadrant.
    pub palette_tr: (u32, u32),
    /// Palette for the bottom-left quadrant.
    pub palette_bl: (u32, u32),
    /// Palette for the bottom-right quadrant.
    pub palette_br: (u32, u32),
}

impl Default for PopArtConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            palette_tl: (0xFF_8A2BE2, 0xFF_FFFF00), // Purple / Yellow
            palette_tr: (0xFF_000080, 0xFF_FFC0CB), // Navy / Pink
            palette_bl: (0xFF_006400, 0xFF_FFA500), // DarkGreen / Orange
            palette_br: (0xFF_8B0000, 0xFF_00FFFF), // DarkRed / Cyan
        }
    }
}

/// Applies a pop-art effect to the framebuffer.
///
/// Reads from `source` and writes to `dest`.
/// `dest` will be divided into 4 quadrants, each displaying a scaled-down
/// version of `source` thresholded with different colors.
/// `source` and `dest` must have the same dimensions.
pub fn apply_pop_art(dest: &mut Framebuffer, source: &Framebuffer, config: &PopArtConfig) {
    if dest.width() != source.width() || dest.height() != source.height() {
        return; // Dimensions must match
    }

    if dest.width() == 0 || dest.height() == 0 {
        return;
    }

    let width = dest.width() as usize;
    let height = dest.height() as usize;
    let half_w = width / 2;
    let half_h = height / 2;

    let src_pixels = source.as_slice();
    let dest_pixels = dest.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        dest_pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .take(height)
            .for_each(|(y, dest_row)| {
                process_row(
                    y, dest_row, src_pixels, width, height, half_w, half_h, config,
                );
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (y, dest_row) in dest_pixels.chunks_exact_mut(width).enumerate().take(height) {
            process_row(
                y, dest_row, src_pixels, width, height, half_w, half_h, config,
            );
        }
    }
}

#[inline(always)]
fn process_row(
    y: usize,
    dest_row: &mut [u32],
    src_pixels: &[u32],
    width: usize,
    height: usize,
    half_w: usize,
    half_h: usize,
    config: &PopArtConfig,
) {
    // Determine which vertical half we're in
    let is_top = y < half_h;

    // Calculate source Y (scale by 2)
    let sy = if is_top {
        (y * 2).min(height - 1)
    } else {
        ((y - half_h) * 2).min(height - 1)
    };
    let src_row_start = sy * width;

    for x in 0..width {
        // Determine which horizontal half we're in
        let is_left = x < half_w;

        // Calculate source X (scale by 2)
        let sx = if is_left {
            (x * 2).min(width - 1)
        } else {
            ((x - half_w) * 2).min(width - 1)
        };

        // Sample source pixel
        let src_color = src_pixels[src_row_start + sx];

        // Calculate luminance
        let r = ((src_color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((src_color >> 8) & 0xFF) as f32 / 255.0;
        let b = (src_color & 0xFF) as f32 / 255.0;
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;

        // Select palette based on quadrant
        let palette = match (is_top, is_left) {
            (true, true) => config.palette_tl,
            (true, false) => config.palette_tr,
            (false, true) => config.palette_bl,
            (false, false) => config.palette_br,
        };

        // Apply thresholding
        dest_row[x] = if luminance < config.threshold {
            palette.0 // Dark color
        } else {
            palette.1 // Light color
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pop_art_mismatched_sizes() {
        let mut dest = Framebuffer::new(10, 10).unwrap();
        let source = Framebuffer::new(5, 5).unwrap();
        // Should return early and not modify dest
        apply_pop_art(&mut dest, &source, &PopArtConfig::default());
        assert_eq!(dest.get_pixel(0, 0).unwrap(), 0xFF00_0000); // Still black
    }

    #[test]
    fn test_apply_pop_art_quadrants() {
        let width = 4;
        let height = 4;
        let mut dest = Framebuffer::new(width, height).unwrap();
        let mut source = Framebuffer::new(width, height).unwrap();

        // Source: left side dark, right side light
        for y in 0..height {
            for x in 0..width {
                if x < 2 {
                    source.set_pixel(x as i32, y as i32, 0xFF_222222); // Dark
                } else {
                    source.set_pixel(x as i32, y as i32, 0xFF_DDDDDD); // Light
                }
            }
        }

        let config = PopArtConfig {
            threshold: 0.5,
            palette_tl: (0xFF_111111, 0xFF_222222),
            palette_tr: (0xFF_333333, 0xFF_444444),
            palette_bl: (0xFF_555555, 0xFF_666666),
            palette_br: (0xFF_777777, 0xFF_888888),
        };

        apply_pop_art(&mut dest, &source, &config);

        // Top-Left (0,0) maps to source left side (dark) -> tl dark color
        assert_eq!(dest.get_pixel(0, 0).unwrap(), 0xFF_111111);
        // Top-Left (1,0) maps to source right side (light) -> tl light color
        assert_eq!(dest.get_pixel(1, 0).unwrap(), 0xFF_222222);

        // Top-Right (2,0) maps to source left side (dark) -> tr dark color
        assert_eq!(dest.get_pixel(2, 0).unwrap(), 0xFF_333333);
        // Top-Right (3,0) maps to source right side (light) -> tr light color
        assert_eq!(dest.get_pixel(3, 0).unwrap(), 0xFF_444444);

        // Bottom-Left (0,2) maps to source left side (dark) -> bl dark color
        assert_eq!(dest.get_pixel(0, 2).unwrap(), 0xFF_555555);
        // Bottom-Left (1,2) maps to source right side (light) -> bl light color
        assert_eq!(dest.get_pixel(1, 2).unwrap(), 0xFF_666666);

        // Bottom-Right (2,2) maps to source left side (dark) -> br dark color
        assert_eq!(dest.get_pixel(2, 2).unwrap(), 0xFF_777777);
        // Bottom-Right (3,2) maps to source right side (light) -> br light color
        assert_eq!(dest.get_pixel(3, 2).unwrap(), 0xFF_888888);
    }
}
