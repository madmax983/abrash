//! Hexagonal Mosaic Post-Processing Filter
//!
//! A post-processing effect that converts the framebuffer into a hexagonal
//! mosaic (honeycomb) pattern.

use crate::framebuffer::Framebuffer;

/// Configuration for the Hex Mosaic effect.
#[derive(Debug, Clone, Copy)]
pub struct HexMosaicConfig {
    /// The size of the hexagonal cells.
    pub cell_size: f32,
    /// The thickness of the border between cells.
    pub border_size: f32,
    /// The color of the border lines.
    pub border_color: u32,
}

/// Applies a hexagonal mosaic filter to the framebuffer.
///
/// This filter divides the image into a grid of hexagonal cells, filling each
/// cell with the color of the pixel at its center. It also draws an optional border
/// around each cell.
///
/// # Panics
///
/// Panics if the internal framebuffer allocation fails due to memory exhaustion or extreme dimensions.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the hex mosaic effect.
pub fn apply_hex_mosaic(fb: &mut Framebuffer, config: &HexMosaicConfig) {
    if config.cell_size <= 1.0 {
        return;
    }

    let cell_size = config.cell_size;
    let border_size = config.border_size;
    let border_color = config.border_color;
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    // Clone the framebuffer manually, since `Framebuffer` might not impl Clone directly
    let mut src_fb = Framebuffer::new(width as u32, height as u32).unwrap();
    src_fb.as_mut_slice().copy_from_slice(fb.as_slice());
    let src_pixels = src_fb.as_slice();
    let pixels = fb.as_mut_slice();

    let r = cell_size;
    let sqrt3 = 1.732_050_8f32;
    let grid_w = 3.0 * r;
    let grid_h = sqrt3 * r;

    let inradius = r * sqrt3 * 0.5;

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let py = y as f32;
                for x in 0..width {
                    let px = x as f32;

                    // Grid 1
                    // ⚡ Bolt: Replace f32::round() with fast integer casting
                    let grid_x1 = (px / grid_w + 0.5) as i32 as f32;
                    let grid_y1 = (py / grid_h + 0.5) as i32 as f32;
                    let cx1 = grid_x1 * grid_w;
                    let cy1 = grid_y1 * grid_h;

                    // Grid 2
                    let grid_x2 = ((px - 1.5 * r) / grid_w + 0.5) as i32 as f32;
                    let grid_y2 = ((py - 0.5 * grid_h) / grid_h + 0.5) as i32 as f32;
                    let cx2 = grid_x2 * grid_w + 1.5 * r;
                    let cy2 = grid_y2 * grid_h + 0.5 * grid_h;

                    let dx1 = px - cx1;
                    let dy1 = py - cy1;
                    let dist1_sq = dx1 * dx1 + dy1 * dy1;

                    let dx2 = px - cx2;
                    let dy2 = py - cy2;
                    let dist2_sq = dx2 * dx2 + dy2 * dy2;

                    let (cx, cy, dx, dy) = if dist1_sq < dist2_sq {
                        (cx1, cy1, dx1, dy1)
                    } else {
                        (cx2, cy2, dx2, dy2)
                    };

                    let center_x = (cx + 0.5) as i32 as usize;
                    let center_x = center_x.clamp(0, width - 1);
                    let center_y = (cy + 0.5) as i32 as usize;
                    let center_y = center_y.clamp(0, height - 1);

                    let abs_dx = dx.abs();
                    let abs_dy = dy.abs();

                    // Distance to nearest hexagon edge
                    let hex_dist = abs_dy.max(abs_dx * (sqrt3 * 0.5) + abs_dy * 0.5);

                    if hex_dist > inradius - border_size {
                        row[x] = border_color;
                    } else {
                        row[x] = src_pixels[center_y * width + center_x];
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let py = y as f32;
            for x in 0..width {
                let px = x as f32;

                // Grid 1
                // ⚡ Bolt: Replace f32::round() with fast integer casting
                let grid_x1 = (px / grid_w + 0.5) as i32 as f32;
                let grid_y1 = (py / grid_h + 0.5) as i32 as f32;
                let cx1 = grid_x1 * grid_w;
                let cy1 = grid_y1 * grid_h;

                // Grid 2
                let grid_x2 = ((px - 1.5 * r) / grid_w + 0.5) as i32 as f32;
                let grid_y2 = ((py - 0.5 * grid_h) / grid_h + 0.5) as i32 as f32;
                let cx2 = grid_x2 * grid_w + 1.5 * r;
                let cy2 = grid_y2 * grid_h + 0.5 * grid_h;

                let dx1 = px - cx1;
                let dy1 = py - cy1;
                let dist1_sq = dx1 * dx1 + dy1 * dy1;

                let dx2 = px - cx2;
                let dy2 = py - cy2;
                let dist2_sq = dx2 * dx2 + dy2 * dy2;

                let (cx, cy, dx, dy) = if dist1_sq < dist2_sq {
                    (cx1, cy1, dx1, dy1)
                } else {
                    (cx2, cy2, dx2, dy2)
                };

                let center_x = (cx + 0.5) as i32 as usize;
                let center_x = center_x.clamp(0, width - 1);
                let center_y = (cy + 0.5) as i32 as usize;
                let center_y = center_y.clamp(0, height - 1);

                let abs_dx = dx.abs();
                let abs_dy = dy.abs();

                // Distance to nearest hexagon edge
                let hex_dist = abs_dy.max(abs_dx * (sqrt3 * 0.5) + abs_dy * 0.5);

                if hex_dist > inradius - border_size {
                    pixels[y * width + x] = border_color;
                } else {
                    pixels[y * width + x] = src_pixels[center_y * width + center_x];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_hex_mosaic() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFF_FFFF);

        apply_hex_mosaic(
            &mut fb,
            &HexMosaicConfig {
                cell_size: 10.0,
                border_size: 1.0,
                border_color: 0xFF00_0000,
            },
        );

        // Ensure the border is drawn (some black pixels should exist)
        let has_black = fb.as_slice().iter().any(|&p| p == 0xFF00_0000);
        assert!(has_black);
    }
}
