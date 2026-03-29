//! Radar Sweep Filter
//!
//! A retro post-processing effect that simulates a sweeping radar screen.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::PI;

thread_local! {
    static RADAR_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub struct RadarConfig {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub angle: f32,
    pub tail_length: f32,
    pub base_color: u32,
    pub sweep_color: u32,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.45,
            angle: 0.0,
            tail_length: PI / 2.0,
            base_color: 0xFF_002200,
            sweep_color: 0xFF_00FF00,
        }
    }
}

pub fn apply_radar(fb: &mut Framebuffer, config: &RadarConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let aspect_ratio = width as f32 / height as f32;
    let inv_width = 1.0 / width as f32;
    let inv_height = 1.0 / height as f32;

    let r_base = ((config.base_color >> 16) & 0xFF) as f32;
    let g_base = ((config.base_color >> 8) & 0xFF) as f32;
    let b_base = (config.base_color & 0xFF) as f32;

    let r_sweep = ((config.sweep_color >> 16) & 0xFF) as f32;
    let g_sweep = ((config.sweep_color >> 8) & 0xFF) as f32;
    let b_sweep = (config.sweep_color & 0xFF) as f32;

    let mut src_fb_vec = RADAR_BUFFER.with(RefCell::take);
    let size = width * height;
    if src_fb_vec.len() < size {
        src_fb_vec.resize(size, 0);
    }
    let src_pixels = &mut src_fb_vec[..size];
    src_pixels.copy_from_slice(fb.as_slice());

    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let v = y as f32 * inv_height;
        let dy = v - config.center_y;

        for (x, pixel) in row.iter_mut().enumerate() {
            let u = x as f32 * inv_width;
            let dx = (u - config.center_x) * aspect_ratio;
            #[allow(clippy::imprecise_flops)]
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > config.radius {
                *pixel = 0xFF_000000;
                continue;
            }

            if (dist - config.radius).abs() < 0.005 {
                *pixel = config.sweep_color;
                continue;
            }

            if dist < 0.005
                || (dx.abs() < 0.002 && dy.abs() < config.radius)
                || (dy.abs() < 0.002 && dx.abs() < config.radius)
            {
                *pixel = config.base_color;
                continue;
            }

            let mut pixel_angle = dy.atan2(dx);
            if pixel_angle < 0.0 {
                pixel_angle += 2.0 * PI;
            }

            let mut sweep_angle = config.angle % (2.0 * PI);
            if sweep_angle < 0.0 {
                sweep_angle += 2.0 * PI;
            }

            let mut angle_diff = sweep_angle - pixel_angle;
            if angle_diff < 0.0 {
                angle_diff += 2.0 * PI;
            }

            let orig = src_pixels[y * width + x];
            let o_r = ((orig >> 16) & 0xFF) as f32;
            let o_g = ((orig >> 8) & 0xFF) as f32;
            let o_b = (orig & 0xFF) as f32;
            let lum = (0.299 * o_r + 0.587 * o_g + 0.114 * o_b) / 255.0;

            if angle_diff <= config.tail_length {
                let intensity = 1.0 - (angle_diff / config.tail_length);
                let r =
                    (r_base + (r_sweep - r_base) * intensity + lum * 50.0).clamp(0.0, 255.0) as u32;
                let g = (g_base + (g_sweep - g_base) * intensity + lum * 255.0).clamp(0.0, 255.0)
                    as u32;
                let b =
                    (b_base + (b_sweep - b_base) * intensity + lum * 50.0).clamp(0.0, 255.0) as u32;
                *pixel = 0xFF_000000 | (r << 16) | (g << 8) | b;
            } else {
                let r = (r_base + lum * 20.0).clamp(0.0, 255.0) as u32;
                let g = (g_base + lum * 100.0).clamp(0.0, 255.0) as u32;
                let b = (b_base + lum * 20.0).clamp(0.0, 255.0) as u32;
                *pixel = 0xFF_000000 | (r << 16) | (g << 8) | b;
            }
        }
    });

    RADAR_BUFFER.with(|buf| {
        buf.replace(src_fb_vec);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_apply_radar() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_FFFFFF);
        let mut config = RadarConfig::default();
        config.angle = PI;
        apply_radar(&mut fb, &config);
        // Verify framebuffer slice is modified (black background, some green)
        let bg = fb.as_slice()[0];
        assert_eq!(bg, 0xFF_000000);
    }
}
