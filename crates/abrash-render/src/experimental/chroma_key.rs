//! Chroma Key processing for background replacement.
//!
//! This module provides functions to composite a foreground [`Framebuffer`] over a
//! background [`Framebuffer`] by removing a specific "key color" (often green or blue).
//! This simulates a "green screen" effect in software rendering.
//!
//! Both an exact match algorithm ([`apply_chroma_key`]) and a soft-edge blending
//! algorithm ([`smooth_chroma_key`]) are provided.

use abrash_core::framebuffer::Framebuffer;

/// Replaces pixels in the foreground framebuffer that exactly match the key color
/// with the corresponding pixels from the background framebuffer.
///
/// This performs a hard replacement. If the pixel color in `fg` is exactly `key_color`,
/// it is overwritten by the pixel at the same coordinates in `bg`.
///
/// If the framebuffers are of different sizes, the operation is limited to the
/// overlapping minimum width and height of the two buffers.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::experimental::chroma_key::apply_chroma_key;
///
/// let mut fg = Framebuffer::new(2, 2).unwrap();
/// let mut bg = Framebuffer::new(2, 2).unwrap();
///
/// // Setup background (e.g., a white wall)
/// bg.clear(0xFF_FFFFFF);
///
/// // Setup foreground with a green screen (0xFF_00FF00) and a red actor
/// fg.set_pixel(0, 0, 0xFF_00FF00); // Green
/// fg.set_pixel(1, 0, 0xFF_FF0000); // Red
///
/// // Apply the exact match chroma key
/// apply_chroma_key(&mut fg, &bg, 0xFF_00FF00);
///
/// // The green pixel is replaced by the white background
/// assert_eq!(fg.get_pixel(0, 0).unwrap(), 0xFF_FFFFFF);
/// // The red pixel remains unchanged
/// assert_eq!(fg.get_pixel(1, 0).unwrap(), 0xFF_FF0000);
/// ```
pub fn apply_chroma_key(fg: &mut Framebuffer, bg: &Framebuffer, key_color: u32) {
    let width = fg.width().min(bg.width()) as usize;
    let height = fg.height().min(bg.height()) as usize;
    let fg_stride = fg.width() as usize;
    let bg_stride = bg.width() as usize;

    let fg_pixels = fg.as_mut_slice();
    let bg_pixels = bg.as_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        fg_pixels
            .par_chunks_exact_mut(fg_stride)
            .zip(bg_pixels.par_chunks_exact(bg_stride))
            .take(height)
            .for_each(|(fg_row, bg_row)| {
                for x in 0..width {
                    if fg_row[x] == key_color {
                        fg_row[x] = bg_row[x];
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let fg_row = &mut fg_pixels[y * fg_stride..(y + 1) * fg_stride];
            let bg_row = &bg_pixels[y * bg_stride..(y + 1) * bg_stride];
            for x in 0..width {
                if fg_row[x] == key_color {
                    fg_row[x] = bg_row[x];
                }
            }
        }
    }
}

/// Composites the foreground over the background using a smooth chroma key algorithm.
///
/// Unlike [`apply_chroma_key`], this function calculates the Euclidean distance in RGB
/// color space between the foreground pixel and the `key_color`.
///
/// - Pixels within `threshold` distance from `key_color` are fully replaced by the background.
/// - Pixels between `threshold` and `threshold + feather` are smoothly alpha blended,
///   reducing hard jagged edges on the subject.
/// - Pixels beyond `threshold + feather` remain untouched.
///
/// If the framebuffers are of different sizes, the operation is limited to the
/// overlapping minimum width and height of the two buffers.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::experimental::chroma_key::smooth_chroma_key;
///
/// let mut fg = Framebuffer::new(2, 2).unwrap();
/// let mut bg = Framebuffer::new(2, 2).unwrap();
///
/// // Background is pure white
/// bg.clear(0xFF_FFFFFF);
///
/// // Foreground has a pure green pixel, a slightly dark green pixel, and a red pixel
/// fg.set_pixel(0, 0, 0xFF_00FF00); // Pure Green (Exact match)
/// fg.set_pixel(1, 0, 0xFF_00AA00); // Dark Green (Will be feathered)
/// fg.set_pixel(0, 1, 0xFF_FF0000); // Red (Untouched)
///
/// // Apply soft chroma key (Threshold = 20.0, Feather = 100.0)
/// smooth_chroma_key(&mut fg, &bg, 0xFF_00FF00, 20.0, 100.0);
///
/// // The exact green match is fully replaced with white
/// assert_eq!(fg.get_pixel(0, 0).unwrap(), 0xFF_FFFFFF);
///
/// // The dark green is partially blended with white
/// let blended = fg.get_pixel(1, 0).unwrap();
/// assert_ne!(blended, 0xFF_00AA00); // Changed
/// assert_ne!(blended, 0xFF_FFFFFF); // But not fully white
///
/// // The red pixel is untouched
/// assert_eq!(fg.get_pixel(0, 1).unwrap(), 0xFF_FF0000);
/// ```
pub fn smooth_chroma_key(
    fg: &mut Framebuffer,
    bg: &Framebuffer,
    key_color: u32,
    threshold: f32,
    feather: f32,
) {
    let width = fg.width().min(bg.width()) as usize;
    let height = fg.height().min(bg.height()) as usize;
    let fg_stride = fg.width() as usize;
    let bg_stride = bg.width() as usize;

    let fg_pixels = fg.as_mut_slice();
    let bg_pixels = bg.as_slice();

    let key_r = ((key_color >> 16) & 0xFF) as f32;
    let key_g = ((key_color >> 8) & 0xFF) as f32;
    let key_b = (key_color & 0xFF) as f32;

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        fg_pixels
            .par_chunks_exact_mut(fg_stride)
            .zip(bg_pixels.par_chunks_exact(bg_stride))
            .take(height)
            .for_each(|(fg_row, bg_row)| {
                process_smooth_row(
                    fg_row, bg_row, width, key_r, key_g, key_b, threshold, feather,
                );
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let fg_row = &mut fg_pixels[y * fg_stride..(y + 1) * fg_stride];
            let bg_row = &bg_pixels[y * bg_stride..(y + 1) * bg_stride];
            process_smooth_row(
                fg_row, bg_row, width, key_r, key_g, key_b, threshold, feather,
            );
        }
    }
}

fn process_smooth_row(
    fg_row: &mut [u32],
    bg_row: &[u32],
    width: usize,
    key_r: f32,
    key_g: f32,
    key_b: f32,
    threshold: f32,
    feather: f32,
) {
    for x in 0..width {
        let fg_pixel = fg_row[x];
        let fg_a = fg_pixel & 0xFF00_0000;
        let fg_r = ((fg_pixel >> 16) & 0xFF) as f32;
        let fg_g = ((fg_pixel >> 8) & 0xFF) as f32;
        let fg_b = (fg_pixel & 0xFF) as f32;

        let dr = fg_r - key_r;
        let dg = fg_g - key_g;
        let db = fg_b - key_b;

        // ⚡ Bolt: Fast exit using squared distance eliminates costly `.sqrt()`
        // for exact or sub-threshold matches (the majority of green screen pixels).
        let dist_sq = dr * dr + dg * dg + db * db;
        let threshold_sq = threshold * threshold;

        if dist_sq <= threshold_sq {
            fg_row[x] = bg_row[x];
        } else if feather > 0.0 {
            // Euclidean distance in RGB space
            let dist = dist_sq.sqrt();
            if dist < threshold + feather {
                // Calculate alpha for blending (0.0 = fully bg, 1.0 = fully fg)
                let alpha = (dist - threshold) / feather;
                let inv_alpha = 1.0 - alpha;

                let bg_pixel = bg_row[x];
                let bg_r = ((bg_pixel >> 16) & 0xFF) as f32;
                let bg_g = ((bg_pixel >> 8) & 0xFF) as f32;
                let bg_b = (bg_pixel & 0xFF) as f32;

                let out_r = (fg_r * alpha + bg_r * inv_alpha) as u32;
                let out_g = (fg_g * alpha + bg_g * inv_alpha) as u32;
                let out_b = (fg_b * alpha + bg_b * inv_alpha) as u32;

                fg_row[x] = fg_a | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_chroma_key() {
        let mut fg = Framebuffer::new(2, 2).unwrap();
        let mut bg = Framebuffer::new(2, 2).unwrap();

        // Key color: pure green
        let key = 0xFF_00FF00;

        // Setup FG: Top-left is green, top-right is red, bottom-left is green, bottom-right is blue
        fg.set_pixel(0, 0, key);
        fg.set_pixel(1, 0, 0xFF_FF0000);
        fg.set_pixel(0, 1, key);
        fg.set_pixel(1, 1, 0xFF_0000FF);

        // Setup BG: all white
        bg.clear(0xFF_FFFFFF);

        apply_chroma_key(&mut fg, &bg, key);

        assert_eq!(fg.get_pixel(0, 0).unwrap(), 0xFF_FFFFFF);
        assert_eq!(fg.get_pixel(1, 0).unwrap(), 0xFF_FF0000);
        assert_eq!(fg.get_pixel(0, 1).unwrap(), 0xFF_FFFFFF);
        assert_eq!(fg.get_pixel(1, 1).unwrap(), 0xFF_0000FF);
    }

    #[test]
    fn test_smooth_chroma_key() {
        let mut fg = Framebuffer::new(2, 2).unwrap();
        let mut bg = Framebuffer::new(2, 2).unwrap();

        // Key color: pure green
        let key = 0xFF_00FF00;

        // Setup FG
        fg.set_pixel(0, 0, 0xFF_00FF00); // Exact key
        fg.set_pixel(1, 0, 0xFF_00EE00); // Close to key
        fg.set_pixel(0, 1, 0xFF_00AA00); // Feathered
        fg.set_pixel(1, 1, 0xFF_FF0000); // Far from key

        // Setup BG: all white
        bg.clear(0xFF_FFFFFF);

        // Threshold = 20, Feather = 100
        smooth_chroma_key(&mut fg, &bg, key, 20.0, 100.0);

        assert_eq!(fg.get_pixel(0, 0).unwrap(), 0xFF_FFFFFF); // Exact match replaced
        assert_eq!(fg.get_pixel(1, 0).unwrap(), 0xFF_FFFFFF); // Within threshold replaced

        // (0, 1) is partially blended, so it shouldn't be fully white or fully green
        let mixed = fg.get_pixel(0, 1).unwrap();
        assert_ne!(mixed, 0xFF_FFFFFF);
        assert_ne!(mixed, 0xFF_00AA00);

        assert_eq!(fg.get_pixel(1, 1).unwrap(), 0xFF_FF0000); // Too far, untouched
    }
}
