use abrash_core::framebuffer::Framebuffer;

/// Replaces pixels in the foreground framebuffer that exactly match the key color
/// with the corresponding pixels from the background framebuffer.
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
/// Pixels within `threshold` distance from `key_color` are fully replaced.
/// Pixels between `threshold` and `threshold + feather` are alpha blended.
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
                process_smooth_row(fg_row, bg_row, width, key_r, key_g, key_b, threshold, feather);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let fg_row = &mut fg_pixels[y * fg_stride..(y + 1) * fg_stride];
            let bg_row = &bg_pixels[y * bg_stride..(y + 1) * bg_stride];
            process_smooth_row(fg_row, bg_row, width, key_r, key_g, key_b, threshold, feather);
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

        // Euclidean distance in RGB space
        let dist = (dr * dr + dg * dg + db * db).sqrt();

        if dist <= threshold {
            fg_row[x] = bg_row[x];
        } else if feather > 0.0 && dist < threshold + feather {
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
