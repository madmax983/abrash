//! Paper Cutout Filter
//!
//! Simulates a layered 2D construction paper art style by taking 3D depth information
//! and flattening it into discrete strata. It applies distinct shadow drops between
//! depth layers to enhance the 2.5D pop-up book effect.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Paper Cutout effect.
#[derive(Debug, Clone)]
pub struct PaperCutoutConfig {
    /// The number of discrete depth layers to quantize the scene into.
    pub layers: u32,
    /// X offset for drop shadows.
    pub shadow_offset_x: i32,
    /// Y offset for drop shadows.
    pub shadow_offset_y: i32,
    /// The opacity/darkness of the drop shadow (0.0 to 1.0).
    pub shadow_opacity: f32,
    /// Outline color around layers (if applicable, use 0 for transparent).
    pub outline_color: u32,
}

impl Default for PaperCutoutConfig {
    fn default() -> Self {
        Self {
            layers: 8,
            shadow_offset_x: 8,
            shadow_offset_y: 12,
            shadow_opacity: 0.5,
            outline_color: 0x0000_0000,
        }
    }
}

/// Applies a paper cutout style effect to the framebuffer based on depth.
///
/// Converts the image into discrete layers based on Z-buffer depth and applies
/// offset shadows where one layer overlaps a deeper layer.
use std::cell::RefCell;

thread_local! {
    static SRC_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static LAYER_BUFFER: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a paper cutout style effect to the framebuffer based on depth.
///
/// Converts the image into discrete layers based on Z-buffer depth and applies
/// offset shadows where one layer overlaps a deeper layer.
pub fn apply_paper_cutout(fb: &mut Framebuffer, zb: &ZBuffer, config: &PaperCutoutConfig) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let len = width * height;

    if len == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // 1. Find min and max depth (excluding Infinity)
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if !has_content {
        return; // Nothing to process
    }

    // Add a small epsilon to avoid division by zero if flat plane
    let range = (max_z - min_z).max(0.0001);

    // Extract buffers from thread local to avoid borrow panics during rayon stealing
    let mut src = SRC_BUFFER.with(|buf| std::mem::take(&mut *buf.borrow_mut()));
    if src.len() < len {
        src.resize(len, 0);
    }
    src[..len].copy_from_slice(pixels);

    let mut layers_arr = LAYER_BUFFER.with(|buf| std::mem::take(&mut *buf.borrow_mut()));
    if layers_arr.len() < len {
        layers_arr.resize(len, 0);
    }

    let layers_count = config.layers.max(1) as f32;
    let inv_range = 1.0 / range;

    let layers_slice = &mut layers_arr[..len];

    // Pre-calculate layers to avoid float math in the shadow loop
    #[cfg(feature = "parallel")]
    {
        layers_slice
            .par_iter_mut()
            .zip(depths.par_iter())
            .for_each(|(l, &d)| {
                *l = if d == f32::INFINITY {
                    layers_count as i32 + 1 // Use +1 for infinity so valid objects at max depth still get quantized
                } else {
                    let normalized = (d - min_z) * inv_range;
                    (normalized * layers_count) as i32
                };
            });
    }
    #[cfg(not(feature = "parallel"))]
    {
        for (l, &d) in layers_slice.iter_mut().zip(depths.iter()) {
            *l = if d == f32::INFINITY {
                layers_count as i32 + 1 // +1 for infinity
            } else {
                let normalized = (d - min_z) * inv_range;
                (normalized * layers_count) as i32
            };
        }
    }

    // Pre-calculate shadow multiplier
    let shadow_mult = 1.0 - config.shadow_opacity.clamp(0.0, 1.0);
    let shadow_mult_fixed = (shadow_mult * 256.0) as u32;

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    // We need immutable slices for the parallel iteration
    let src_slice = &src[..len];
    let layers_read_slice = &layers_arr[..len];

    iter.for_each(|(y, row)| {
        let row_offset = y * width;
        let shadow_y = y as i32 - config.shadow_offset_y;

        for (x, pixel) in row.iter_mut().enumerate() {
            let idx = row_offset + x;
            let current_layer = layers_read_slice[idx];

            // Read the source color for this pixel
            let mut color = src_slice[idx];

            // Quantize the color to simulate flat paper
            // We reduce the color depth to simulate construction paper limited palette
            if current_layer != layers_count as i32 + 1 {
                // if not infinity
                let r = ((color >> 16) & 0xFF) & 0xE0; // Keep top 3 bits
                let g = ((color >> 8) & 0xFF) & 0xE0;
                let b = (color & 0xFF) & 0xE0;
                // Add some brightness back to compensate for truncation
                let r = r | (r >> 3);
                let g = g | (g >> 3);
                let b = b | (b >> 3);
                color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }

            // Check if this pixel is in a shadow cast by a shallower layer
            let shadow_x = x as i32 - config.shadow_offset_x;

            if shadow_x >= 0 && shadow_x < width as i32 && shadow_y >= 0 && shadow_y < height as i32
            {
                let caster_idx = shadow_y as usize * width + shadow_x as usize;
                let caster_layer = layers_read_slice[caster_idx];

                // If the layer casting the shadow is shallower (closer to camera / lower index)
                // than the current pixel's layer, apply shadow.
                if caster_layer < current_layer {
                    let r = ((color >> 16) & 0xFF) * shadow_mult_fixed / 256;
                    let g = ((color >> 8) & 0xFF) * shadow_mult_fixed / 256;
                    let b = (color & 0xFF) * shadow_mult_fixed / 256;
                    color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
            }

            *pixel = color;
        }
    });

    // Restore buffers
    SRC_BUFFER.with(|buf| *buf.borrow_mut() = src);
    LAYER_BUFFER.with(|buf| *buf.borrow_mut() = layers_arr);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_paper_cutout_basic() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        let mut zb = ZBuffer::new(20, 20).unwrap();

        // Background is white
        fb.clear(0xFFFF_FFFF);

        // Draw a mid-depth square
        for y in 5..15 {
            for x in 5..15 {
                fb.set_pixel(x, y, 0xFF00_FF00); // Green square
                zb.test_and_set(x, y, 10.0);
            }
        }

        let mut config = PaperCutoutConfig::default();
        config.shadow_offset_x = 2;
        config.shadow_offset_y = 2;
        config.shadow_opacity = 0.5;

        apply_paper_cutout(&mut fb, &zb, &config);

        // The square itself should still be mostly green (quantized)
        let sq_px = fb.get_pixel(10, 10).unwrap();
        assert_eq!((sq_px >> 24) & 0xFF, 0xFF);
        assert!((sq_px >> 8) & 0xFF > 0); // Green channel still has value

        // Shadow should be cast at (15..17, 5..15) and (5..17, 15..17)
        // Check pixel to the bottom right of the square
        let shadow_x = 15;
        let shadow_y = 15;

        let shadow_px = fb.get_pixel(shadow_x, shadow_y).unwrap();
        // Original background was white, should be darkened
        assert_ne!(shadow_px, 0xFFFF_FFFF);

        let r = (shadow_px >> 16) & 0xFF;
        let g = (shadow_px >> 8) & 0xFF;
        let b = shadow_px & 0xFF;

        // With 0.5 opacity, white (255) becomes ~127
        assert!(r > 100 && r < 150);
        assert!(g > 100 && g < 150);
        assert!(b > 100 && b < 150);
    }

    #[test]
    fn test_apply_paper_cutout_empty_buffer() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let zb = ZBuffer::new(0, 0).unwrap();
        let config = PaperCutoutConfig::default();
        apply_paper_cutout(&mut fb, &zb, &config);
    }

    #[test]
    fn test_apply_paper_cutout_no_depth() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        let zb = ZBuffer::new(20, 20).unwrap();
        let config = PaperCutoutConfig::default();
        fb.clear(0xFFFF_FFFF);
        apply_paper_cutout(&mut fb, &zb, &config);
        assert_eq!(fb.get_pixel(10, 10).unwrap(), 0xFFFF_FFFF);
    }
}
