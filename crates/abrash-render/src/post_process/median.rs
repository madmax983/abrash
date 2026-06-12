//! Median Filter Effect
//!
//! Applies a median filter to the framebuffer to reduce noise.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Median filter.
#[derive(Debug, Clone, Copy)]
pub struct MedianFilterConfig {
    /// The radius of the median filter. A radius of 1 means a 3x3 kernel.
    pub radius: usize,
}

impl Default for MedianFilterConfig {
    fn default() -> Self {
        Self { radius: 1 }
    }
}

/// Applies a median filter to the framebuffer to reduce noise while preserving edges.
pub fn apply_median_filter(fb: &mut Framebuffer, config: &MedianFilterConfig) {
    if config.radius == 0 || fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let r = config.radius as isize;
    let window_size = (2 * config.radius + 1) * (2 * config.radius + 1);

    // Read-only source pixels
    let src = fb.as_slice();
    // Temporary buffer to avoid reading modified pixels
    let mut dst = vec![0u32; width * height];

    // We will extract color channels, sort, and reconstruct
    for y in 0..height {
        for x in 0..width {
            let mut r_vals = Vec::with_capacity(window_size);
            let mut g_vals = Vec::with_capacity(window_size);
            let mut b_vals = Vec::with_capacity(window_size);

            for dy in -r..=r {
                let ny = y as isize + dy;
                if ny < 0 || ny >= height as isize { continue; }

                for dx in -r..=r {
                    let nx = x as isize + dx;
                    if nx < 0 || nx >= width as isize { continue; }

                    let idx = (ny * width as isize + nx) as usize;
                    let color = src[idx];

                    r_vals.push((color >> 16) & 0xFF);
                    g_vals.push((color >> 8) & 0xFF);
                    b_vals.push(color & 0xFF);
                }
            }

            r_vals.sort_unstable();
            g_vals.sort_unstable();
            b_vals.sort_unstable();

            let mid = r_vals.len() / 2;
            let med_r = r_vals[mid];
            let med_g = g_vals[mid];
            let med_b = b_vals[mid];

            dst[y * width + x] = 0xFF00_0000 | (med_r << 16) | (med_g << 8) | med_b;
        }
    }

    fb.as_mut_slice().copy_from_slice(&dst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_median_filter() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // A single bright white pixel in a sea of black (salt noise)
        fb.set_pixel(1, 1, 0xFFFF_FFFF);

        let config = MedianFilterConfig { radius: 1 };
        apply_median_filter(&mut fb, &config);

        // The bright white pixel should be filtered out by the surrounding black pixels
        let p = fb.get_pixel(1, 1).unwrap();
        assert_eq!(p, 0xFF00_0000);
    }
}
