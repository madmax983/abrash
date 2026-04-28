//! Hexagonal Mosaic Filter
//!
//! Applies a hexagonal pixelation effect to the framebuffer.
//! "Hexagons are the bestagons."

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Hexagonal Mosaic filter.
#[derive(Debug, Clone, Copy)]
pub struct HexMosaicConfig {
    /// Radius of the hexagons.
    pub radius: f32,
    /// Whether to draw borders around the hexagons.
    pub draw_borders: bool,
    /// Color of the hexagon borders (ARGB).
    pub border_color: u32,
    /// Thickness of the hexagon borders.
    pub border_thickness: f32,
}

impl Default for HexMosaicConfig {
    fn default() -> Self {
        Self {
            radius: 10.0,
            draw_borders: true,
            border_color: 0xFF_000000,
            border_thickness: 1.0,
        }
    }
}

/// Applies a hexagonal pixelation mosaic effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the hex mosaic effect.
pub fn apply_hex_mosaic(fb: &mut Framebuffer, config: &HexMosaicConfig) {
    if config.radius <= 1.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let r = config.radius;
    let w = r * 3.0_f32.sqrt();
    let h_spacing = r * 1.5;

    // We need to read from the original and write to it.
    // To avoid read/write tearing, we clone the framebuffer.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut original_pixels = buf.borrow_mut();
        original_pixels.clear();
        original_pixels.extend_from_slice(fb.as_slice());
        let src_pixels = original_pixels.as_slice();

        let pixels = fb.as_mut_slice();

        // parallel chunk processing
        #[cfg(feature = "parallel")]
        let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let fy = y as f32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let fx = x as f32;

                // Grid 1
                let x1 = (fx / w).round() * w;
                let y1 = (fy / (2.0 * h_spacing)).round() * 2.0 * h_spacing;
                let dx1 = fx - x1;
                let dy1 = fy - y1;
                let dist1_sq = dx1 * dx1 + dy1 * dy1;

                // Grid 2
                let x2 = ((fx - w * 0.5) / w).round() * w + w * 0.5;
                let y2 =
                    ((fy - h_spacing) / (2.0 * h_spacing)).round() * 2.0 * h_spacing + h_spacing;
                let dx2 = fx - x2;
                let dy2 = fy - y2;
                let dist2_sq = dx2 * dx2 + dy2 * dy2;

                let (cx, cy, d_min, d_max) = if dist1_sq < dist2_sq {
                    (x1, y1, dist1_sq.sqrt(), dist2_sq.sqrt())
                } else {
                    (x2, y2, dist2_sq.sqrt(), dist1_sq.sqrt())
                };

                if config.draw_borders && (d_max - d_min) < config.border_thickness {
                    *pixel = config.border_color;
                } else {
                    let mut px = cx as i32;
                    let mut py = cy as i32;
                    px = px.clamp(0, width as i32 - 1);
                    py = py.clamp(0, height as i32 - 1);

                    *pixel = src_pixels[py as usize * width + px as usize];
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_hex_mosaic() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF_FFFFFF);
        fb.set_pixel(10, 10, 0xFF_FF0000);

        let config = HexMosaicConfig {
            radius: 5.0,
            draw_borders: false,
            ..Default::default()
        };

        apply_hex_mosaic(&mut fb, &config);
        assert_eq!(fb.width(), 20);
    }
}
