//! Vectorscope Post-Processing Overlay
//!
//! Analyzes the colors in the framebuffer and draws a Vectorscope diagram
//! (color scatter plot mapped by Hue and Saturation) overlaid on the image.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

/// Configuration for the Vectorscope overlay.
pub struct VectorscopeConfig {
    /// Center X of the overlay on the screen
    pub center_x: i32,
    /// Center Y of the overlay on the screen
    pub center_y: i32,
    /// Radius of the vectorscope diagram
    pub radius: i32,
    /// How much a single pixel contributes to the diagram's brightness
    pub intensity: f32,
    /// Draw a background circle
    pub draw_background: bool,
    /// Draw crosshairs
    pub draw_grid: bool,
}

impl Default for VectorscopeConfig {
    fn default() -> Self {
        Self {
            center_x: 120,
            center_y: 120,
            radius: 100,
            intensity: 0.05,
            draw_background: true,
            draw_grid: true,
        }
    }
}

/// Applies a Vectorscope overlay to the framebuffer.
/// Reads all pixels, maps them to a U/V chrominance plane, and renders the result.
pub fn apply_vectorscope(fb: &mut Framebuffer, config: &VectorscopeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.radius <= 0 {
        return;
    }

    let r_size = (config.radius * 2) as usize;

    // Thread-local cache for the vectorscope accumulation buffer to avoid allocations
    thread_local! {
        static SCOPE_BUFFER: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    }

    SCOPE_BUFFER.with(|buf| {
        let mut scope = buf.borrow_mut();
        let total_cells = r_size * r_size;
        if scope.len() == total_cells {
            scope.fill(0.0);
        } else {
            scope.resize(total_cells, 0.0);
        }

        let pixels = fb.as_slice();

        // Analyze framebuffer
        for &pixel in pixels {
            let r = ((pixel >> 16) & 0xFF) as f32 / 255.0;
            let g = ((pixel >> 8) & 0xFF) as f32 / 255.0;
            let b = (pixel & 0xFF) as f32 / 255.0;

            // Simplified RGB to YUV (chrominance only)
            // U = -0.14713 * R - 0.28886 * G + 0.436 * B
            // V = 0.615 * R - 0.51499 * G - 0.10001 * B
            let u = -0.147 * r - 0.289 * g + 0.436 * b;
            let v = 0.615 * r - 0.515 * g - 0.100 * b;

            // Map U/V (-0.5 to 0.5 roughly) to scope radius
            let scope_x = (u * 2.0 * config.radius as f32) as i32 + config.radius;
            let scope_y = (-v * 2.0 * config.radius as f32) as i32 + config.radius;

            if scope_x >= 0 && scope_x < r_size as i32 && scope_y >= 0 && scope_y < r_size as i32 {
                let idx = scope_y as usize * r_size + scope_x as usize;
                scope[idx] += config.intensity;
            }
        }

        // Render to framebuffer
        let cx = config.center_x;
        let cy = config.center_y;
        let r_sq = config.radius * config.radius;

        for sy in 0..r_size as i32 {
            for sx in 0..r_size as i32 {
                let dx = sx - config.radius;
                let dy = sy - config.radius;

                // Only draw inside the circular scope
                if dx * dx + dy * dy <= r_sq {
                    let screen_x = cx - config.radius + sx;
                    let screen_y = cy - config.radius + sy;

                    if screen_x >= 0
                        && screen_x < width as i32
                        && screen_y >= 0
                        && screen_y < height as i32
                    {
                        let idx = sy as usize * r_size + sx as usize;
                        let val = scope[idx];

                        let (mut r, mut g, mut b) = if config.draw_background {
                            (0.1, 0.1, 0.1)
                        } else {
                            (0.0, 0.0, 0.0)
                        };

                        if config.draw_grid && (dx == 0 || dy == 0) {
                            r = 0.3;
                            g = 0.3;
                            b = 0.3;
                        }

                        if val > 0.0 {
                            g += val;
                            r += val * 0.2;
                            b += val * 0.2;
                        }

                        // If not drawing background and no trace/grid, skip
                        if !config.draw_background && r == 0.0 && g == 0.0 && b == 0.0 {
                            continue;
                        }

                        let r_byte = (r * 255.0).clamp(0.0, 255.0) as u32;
                        let g_byte = (g * 255.0).clamp(0.0, 255.0) as u32;
                        let b_byte = (b * 255.0).clamp(0.0, 255.0) as u32;

                        let pixel_idx = screen_y as usize * width + screen_x as usize;
                        fb.as_mut_slice()[pixel_idx] =
                            0xFF00_0000 | (r_byte << 16) | (g_byte << 8) | b_byte;
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
    fn test_apply_vectorscope() {
        let mut fb = Framebuffer::new(200, 200).unwrap();
        // Fill with some red
        fb.clear(0xFFFF0000);

        let config = VectorscopeConfig {
            center_x: 100,
            center_y: 100,
            radius: 50,
            intensity: 0.1,
            draw_background: true,
            draw_grid: true,
        };

        apply_vectorscope(&mut fb, &config);

        // Vectorscope should have modified pixels around the center
        let center_pixel = fb.get_pixel(100, 100).unwrap();
        assert_ne!(center_pixel, 0xFFFF0000); // Modified by background/grid
    }
}
