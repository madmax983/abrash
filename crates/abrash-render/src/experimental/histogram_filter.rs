use crate::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy)]
pub struct HistogramConfig {
    pub width: u32,
    pub height: u32,
    pub color: u32,
    pub bg_color: u32,
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 100,
            color: 0xFF_00FF00,
            bg_color: 0xFF_000000,
        }
    }
}

pub fn apply_histogram(fb: &mut Framebuffer, config: &HistogramConfig) {
    if config.width == 0 || config.height == 0 {
        return;
    }

    let fb_width = fb.width() as usize;
    let fb_height = fb.height() as usize;

    if fb_width == 0 || fb_height == 0 {
        return;
    }

    let mut bins = [0usize; 256];
    let mut max_count = 0usize;

    for &pixel in fb.as_slice() {
        let r = ((pixel >> 16) & 0xFF) as usize;
        let g = ((pixel >> 8) & 0xFF) as usize;
        let b = (pixel & 0xFF) as usize;

        let lum = (r * 77 + g * 150 + b * 29) >> 8;
        bins[lum] += 1;
        if bins[lum] > max_count {
            max_count = bins[lum];
        }
    }

    if max_count == 0 {
        return;
    }

    let start_x = 10.min(fb_width.saturating_sub(config.width as usize));
    let start_y = 10.min(fb_height.saturating_sub(config.height as usize));

    let end_x = start_x + config.width as usize;
    let end_y = start_y + config.height as usize;

    if end_x > fb_width || end_y > fb_height {
        return;
    }

    let pixels = fb.as_mut_slice();

    // Draw background
    for y in start_y..end_y {
        for x in start_x..end_x {
            pixels[y * fb_width + x] = config.bg_color;
        }
    }

    // Draw histogram
    for x in 0..config.width as usize {
        let bin_idx = (x * 255) / config.width as usize;
        let count = bins[bin_idx];
        let h = (count * config.height as usize) / max_count;

        for y in 0..h {
            let px = start_x + x;
            let py = end_y - 1 - y;
            pixels[py * fb_width + px] = config.color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_histogram() {
        let mut fb = Framebuffer::new(300, 200).unwrap();
        fb.clear(0xFF_808080);

        let config = HistogramConfig::default();
        apply_histogram(&mut fb, &config);

        // Check if there is some green in the histogram area
        let mut found = false;
        for y in 0..config.height as usize {
            for x in 0..config.width as usize {
                if fb.get_pixel((x + 10) as i32, (10 + y) as i32).unwrap() == config.color {
                    found = true;
                    break;
                }
            }
        }
        assert!(found);
    }
}
