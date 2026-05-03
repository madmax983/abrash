//! Cel Shading (Toon Shading) post-processing effect.
//!
//! Applies a stylized comic book / anime aesthetic by quantizing colors
//! and drawing edge outlines based on depth and luminance discontinuities.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;

use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Cel Shading effect.
#[derive(Debug, Clone, Copy)]
pub struct CelShadeConfig {
    /// Number of color bands to quantize into (e.g., 4 or 5).
    pub levels: u32,
    /// Sensitivity for edge detection (lower means more edges). Typical: 0.1 to 0.3.
    pub edge_threshold: f32,
    /// The color used for drawing outlines (edges).
    pub edge_color: u32,
}

impl Default for CelShadeConfig {
    fn default() -> Self {
        Self {
            levels: 4,
            edge_threshold: 0.2,
            edge_color: 0xFF_000000,
        }
    }
}

/// Applies a Cel Shading (Toon Shading) effect to the framebuffer.
///
/// This effect is entirely screen-space. It posterizes the colors in the
/// framebuffer to flat bands, and then uses a Sobel edge detection filter
/// on the Z-Buffer and Luminance to find object outlines and creases, drawing
/// them in a solid color (typically black).
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The z-buffer containing depth information for edge detection.
/// * `config` - Configuration for the cel shading effect.
pub fn apply_cel_shade(fb: &mut Framebuffer, zb: &ZBuffer, config: &CelShadeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width < 3 || height < 3 {
        return; // Too small for Sobel
    }

    let levels = config.levels.max(1);
    let edge_thresh = config.edge_threshold;
    let edge_color = config.edge_color;

    SOURCE_PIXELS.with(|source_pixels_cell| {
        let mut source_pixels = source_pixels_cell.borrow_mut();
        let fb_slice = fb.as_slice();

        // ⚡ Bolt: Eliminate `.to_vec()` and instead dynamically resize the thread-local buffer
        // to match the framebuffer length. This reuses memory capacity across frames.
        if source_pixels.len() != fb_slice.len() {
            source_pixels.resize(fb_slice.len(), 0);
        }
        source_pixels.copy_from_slice(fb_slice);

        // Extract as immutable reference to pass into the parallel iterator safely
        let src_pixels: &[u32] = &source_pixels;

        let depths = zb.as_slice();
        let dest_pixels = fb.as_mut_slice();

        // Sobel kernels
        // GX = [-1, 0, 1]   GY = [ 1,  2,  1]
        //      [-2, 0, 2]        [ 0,  0,  0]
        //      [-1, 0, 1]        [-1, -2, -1]

        let process_row = |y: usize, dest_row: &mut [u32]| {
            for (x, pixel_out) in dest_row.iter_mut().enumerate() {
                if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                    // Keep original (quantized) at the very border to avoid OOB
                    *pixel_out = quantize_color(src_pixels[y * width + x], levels);
                    continue;
                }

                let center_idx = y * width + x;

                // 1. Edge Detection (Sobel on Depth)
                let mut gx_z = 0.0;
                let mut gy_z = 0.0;
                let mut gx_l = 0.0;
                let mut gy_l = 0.0;

                // Compute indices once
                let t_idx = center_idx - width;
                let b_idx = center_idx + width;

                let tl = t_idx - 1;
                let tc = t_idx;
                let tr = t_idx + 1;
                let ml = center_idx - 1; /* mc */
                let mr = center_idx + 1;
                let bl = b_idx - 1;
                let bc = b_idx;
                let br = b_idx + 1;

                // Collect Depths (Handling infinity)
                let d_tl = extract_depth(depths[tl]);
                let d_tc = extract_depth(depths[tc]);
                let d_tr = extract_depth(depths[tr]);
                let d_ml = extract_depth(depths[ml]);
                let d_mr = extract_depth(depths[mr]);
                let d_bl = extract_depth(depths[bl]);
                let d_bc = extract_depth(depths[bc]);
                let d_br = extract_depth(depths[br]);

                gx_z += -d_tl + d_tr;
                gx_z += -2.0 * d_ml + 2.0 * d_mr;
                gx_z += -d_bl + d_br;

                gy_z += d_tl + 2.0 * d_tc + d_tr;
                gy_z += -d_bl - 2.0 * d_bc - d_br;

                #[allow(clippy::imprecise_flops)]
                // ⚡ Bolt: Use squared magnitude to avoid f32::sqrt() in hot inner loop
                let edge_z_sq = gx_z * gx_z + gy_z * gy_z;

                // Collect Luminance
                let l_tl = get_lum(src_pixels[tl]);
                let l_tc = get_lum(src_pixels[tc]);
                let l_tr = get_lum(src_pixels[tr]);
                let l_ml = get_lum(src_pixels[ml]);
                let l_mr = get_lum(src_pixels[mr]);
                let l_bl = get_lum(src_pixels[bl]);
                let l_bc = get_lum(src_pixels[bc]);
                let l_br = get_lum(src_pixels[br]);

                gx_l += -l_tl + l_tr;
                gx_l += -2.0 * l_ml + 2.0 * l_mr;
                gx_l += -l_bl + l_br;

                gy_l += l_tl + 2.0 * l_tc + l_tr;
                gy_l += -l_bl - 2.0 * l_bc - l_br;

                #[allow(clippy::imprecise_flops)]
                let edge_l_sq = gx_l * gx_l + gy_l * gy_l;

                // Combine edge strengths
                // Depth edges are strong, luminance edges help with interior creases
                let combined_edge_sq = edge_z_sq + edge_l_sq * 0.25;

                if combined_edge_sq > edge_thresh * edge_thresh {
                    *pixel_out = edge_color;
                } else {
                    *pixel_out = quantize_color(src_pixels[center_idx], levels);
                }
            }
        };

        #[cfg(feature = "parallel")]
        {
            dest_pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    process_row(y, row);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    process_row(y, row);
                });
        }
    });
}

#[inline(always)]
fn extract_depth(z: f32) -> f32 {
    if z.is_infinite() {
        1.0 // Map infinity to 1.0 (Far plane)
    } else {
        // Map typical NDC depth [-1, 1] to [0, 1] for stable sobel gradients
        (z * 0.5 + 0.5).clamp(0.0, 1.0)
    }
}

#[inline(always)]
fn get_lum(color: u32) -> f32 {
    f32::from(pixel_luminance(color)) / 255.0
}

#[inline(always)]
fn quantize_color(color: u32, levels: u32) -> u32 {
    if levels <= 1 {
        return color; // No quantization if levels is 1 or less
    }

    let factor = 255.0 / (levels - 1) as f32;
    let inv_factor = (levels - 1) as f32 / 255.0;

    let a = color & 0xFF00_0000;
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;

    // ⚡ Bolt: Replace f32::round() with fast integer casting
    let qr = (((r * inv_factor + 0.5) as i32 as f32) * factor).clamp(0.0, 255.0) as u32;
    let qg = (((g * inv_factor + 0.5) as i32 as f32) * factor).clamp(0.0, 255.0) as u32;
    let qb = (((b * inv_factor + 0.5) as i32 as f32) * factor).clamp(0.0, 255.0) as u32;

    a | (qr << 16) | (qg << 8) | qb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_color() {
        // 2 levels -> 0 or 255
        assert_eq!(quantize_color(0xFF_000000, 2), 0xFF_000000);
        assert_eq!(quantize_color(0xFF_FFFFFF, 2), 0xFF_FFFFFF);
        assert_eq!(quantize_color(0xFF_808080, 2), 0xFF_FFFFFF); // 128 rounds up to 255

        // 3 levels -> 0, 127, 255
        assert_eq!(quantize_color(0xFF_404040, 3), 0xFF_7F7F7F);
        assert_eq!(quantize_color(0xFF_808080, 3), 0xFF_7F7F7F); // 128 maps to ~128
        assert_eq!(quantize_color(0xFF_C0C0C0, 3), 0xFF_FFFFFF);
    }

    #[test]
    fn test_apply_cel_shade() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();

        // Fill half with a color and depth
        for y in 0..10 {
            for x in 0..5 {
                fb.set_pixel(x, y, 0xFF_AAAAAA);
                zb.test_and_set(x, y, -0.5);
            }
            for x in 5..10 {
                fb.set_pixel(x, y, 0xFF_555555);
                zb.test_and_set(x, y, 0.5);
            }
        }

        let config = CelShadeConfig {
            levels: 4,
            edge_threshold: 0.1,
            edge_color: 0xFF_000000,
        };

        apply_cel_shade(&mut fb, &zb, &config);

        // The boundary at x=5 should have detected an edge and colored it black
        let mut found_edge = false;
        for y in 1..9 {
            if fb.get_pixel(4, y).unwrap() == 0xFF_000000
                || fb.get_pixel(5, y).unwrap() == 0xFF_000000
            {
                found_edge = true;
                break;
            }
        }

        assert!(found_edge, "Should have detected a depth/luminance edge");
    }
}
