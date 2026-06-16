use abrash_core::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy)]
pub struct HalationConfig {
    /// Minimum luminance [0.0..1.0] to trigger halation
    pub threshold: f32,
    /// Size of the blur radius (e.g. 5)
    pub radius: usize,
    /// Intensity of the halation effect (e.g. 1.0)
    pub intensity: f32,
    /// Color multiplier for the halation glow (e.g., strong red and some green for orange glow).
    /// Default typically represents (1.0, 0.2, 0.0) -> Red-orange.
    pub tint: (f32, f32, f32),
}

impl Default for HalationConfig {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            radius: 8,
            intensity: 1.0,
            tint: (1.0, 0.3, 0.0), // Red-Orange
        }
    }
}

pub fn apply_halation(fb: &mut Framebuffer, config: HalationConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_slice();

    // 1. Isolate bright pixels into a temporary buffer
    let mut bright_pixels = vec![0xFF00_0000; width * height];

    // Convert threshold to 0-255 luminance threshold
    let threshold_u8 = (config.threshold * 255.0).clamp(0.0, 255.0) as u32;

    for (src_row, dst_row) in pixels
        .chunks_exact(width)
        .zip(bright_pixels.chunks_exact_mut(width))
    {
        for (x, &color) in src_row.iter().enumerate() {
            // Extract RGB
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;

            // Calculate luminance (approximate)
            let luminance = (r * 299 + g * 587 + b * 114) / 1000;

            if luminance >= threshold_u8 {
                dst_row[x] = color;
            }
        }
    }

    // 2. Apply a horizontal box blur (radius)
    let mut h_blur = vec![0xFF00_0000; width * height];
    let radius = config.radius;

    for (y, src_row) in bright_pixels.chunks_exact(width).enumerate() {
        let dst_row = unsafe { h_blur.get_unchecked_mut(y * width..(y + 1) * width) };
        for x in 0..width {
            let mut r_sum = 0;
            let mut g_sum = 0;
            let mut b_sum = 0;
            let mut count = 0;

            let start_x = x.saturating_sub(radius);
            let end_x = (x + radius).min(width - 1);

            let row_slice = unsafe { src_row.get_unchecked(start_x..=end_x) };
            for &c in row_slice {
                r_sum += (c >> 16) & 0xFF;
                g_sum += (c >> 8) & 0xFF;
                b_sum += c & 0xFF;
                count += 1;
            }

            if count > 0 {
                let r = r_sum / count;
                let g = g_sum / count;
                let b = b_sum / count;
                dst_row[x] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    // 3. Apply a vertical box blur + tint + blend back to original
    let dest_pixels = fb.as_mut_slice();
    let tint_r = (config.tint.0 * 256.0 * config.intensity) as u32;
    let tint_g = (config.tint.1 * 256.0 * config.intensity) as u32;
    let tint_b = (config.tint.2 * 256.0 * config.intensity) as u32;

    for y in 0..height {
        let start_y = y.saturating_sub(radius);
        let end_y = (y + radius).min(height - 1);
        let count = (end_y - start_y + 1) as u32;

        let dst_row = unsafe { dest_pixels.get_unchecked_mut(y * width..(y + 1) * width) };

        for x in 0..width {
            let mut r_sum = 0;
            let mut g_sum = 0;
            let mut b_sum = 0;

            for ny in start_y..=end_y {
                let c = unsafe { *h_blur.get_unchecked(ny * width + x) };
                r_sum += (c >> 16) & 0xFF;
                g_sum += (c >> 8) & 0xFF;
                b_sum += c & 0xFF;
            }

            if count > 0 {
                let r = r_sum / count;
                let g = g_sum / count;
                let b = b_sum / count;

                // Apply tint
                let glow_r = (r * tint_r) >> 8;
                let glow_g = (g * tint_g) >> 8;
                let glow_b = (b * tint_b) >> 8;

                if glow_r > 0 || glow_g > 0 || glow_b > 0 {
                    let base_c = dst_row[x];
                    let base_r = (base_c >> 16) & 0xFF;
                    let base_g = (base_c >> 8) & 0xFF;
                    let base_b = base_c & 0xFF;

                    // Additive blend
                    let out_r = (base_r + glow_r).min(255);
                    let out_g = (base_g + glow_g).min(255);
                    let out_b = (base_b + glow_b).min(255);

                    dst_row[x] = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_halation_dark_pixels_unchanged() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Set all pixels to very dark
        for p in fb.as_mut_slice() {
            *p = 0xFF10_1010;
        }

        let config = HalationConfig {
            threshold: 0.8,
            ..Default::default()
        };

        apply_halation(&mut fb, config);

        // They should remain exactly 0xFF10_1010
        assert_eq!(fb.get_pixel(1, 1), Some(0xFF10_1010));
    }

    #[test]
    fn test_apply_halation_bright_pixels_bleed() {
        let mut fb = Framebuffer::new(5, 5).unwrap();

        // Fill with black
        fb.clear(0xFF00_0000);

        // Center pixel is very bright white
        fb.set_pixel(2, 2, 0xFFFF_FFFF);

        let config = HalationConfig {
            threshold: 0.8,
            radius: 2,
            intensity: 1.0,
            tint: (1.0, 0.0, 0.0), // Pure red tint
        };

        apply_halation(&mut fb, config);

        // Center pixel should still be bright
        let center = fb.get_pixel(2, 2).unwrap();
        assert_ne!(center, 0xFF00_0000);

        // Neighbor pixels should now have some red bleed
        let neighbor_r = fb.get_pixel(3, 2).unwrap();
        let r = (neighbor_r >> 16) & 0xFF;
        let g = (neighbor_r >> 8) & 0xFF;
        let b = neighbor_r & 0xFF;

        assert!(r > 0, "Neighbor should have red bleed from center pixel");
        assert_eq!(
            g, 0,
            "Neighbor should not have green bleed for pure red tint"
        );
        assert_eq!(
            b, 0,
            "Neighbor should not have blue bleed for pure red tint"
        );
    }
}
