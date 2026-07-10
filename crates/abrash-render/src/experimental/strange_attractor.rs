//! Strange Attractor (Clifford Attractor) Generator.
//!
//! Renders mathematical strange attractors iteratively to the framebuffer,
//! generating beautiful, chaotic particle trails purely through simple mathematical functions.

use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Clifford Attractor.
#[derive(Debug, Clone)]
pub struct CliffordConfig {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub iters: usize,
    pub scale: f64,
    pub color: u32,
    pub trail_fade: u8,
}

impl Default for CliffordConfig {
    fn default() -> Self {
        Self {
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            iters: 10_000_000,
            scale: 0.15,
            color: 0x11_FF_AA_00,
            trail_fade: 5,
        }
    }
}

pub struct CliffordAttractor {
    pub x: f64,
    pub y: f64,
}

impl Default for CliffordAttractor {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl CliffordAttractor {
    /// Fades the current framebuffer, preserving trails instead of clearing.
    pub fn fade_framebuffer(fb: &mut Framebuffer, amount: u8) {
        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = pixels.par_iter_mut();
        #[cfg(not(feature = "parallel"))]
        let iter = pixels.iter_mut();

        iter.for_each(|pixel| {
            let p = *pixel;
            let a = (p >> 24) & 0xFF;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            let r_new = r.saturating_sub(u32::from(amount));
            let g_new = g.saturating_sub(u32::from(amount));
            let b_new = b.saturating_sub(u32::from(amount));

            *pixel = (a << 24) | (r_new << 16) | (g_new << 8) | b_new;
        });
    }

    /// Steps the attractor, accumulating points on the framebuffer.
    pub fn render(&mut self, fb: &mut Framebuffer, config: &CliffordConfig) {
        let width = f64::from(fb.width());
        let height = f64::from(fb.height());
        let half_w = width * 0.5;
        let half_h = height * 0.5;

        for _ in 0..config.iters {
            let next_x = (config.a * self.y).sin() + config.c * (config.a * self.x).cos();
            let next_y = (config.b * self.x).sin() + config.d * (config.b * self.y).cos();

            self.x = next_x;
            self.y = next_y;

            let px = (half_w + self.x * width * config.scale) as i32;
            let py = (half_h + self.y * height * config.scale) as i32;

            if let Some(existing) = fb.get_pixel(px, py) {
                let r1 = (existing >> 16) & 0xFF;
                let g1 = (existing >> 8) & 0xFF;
                let b1 = existing & 0xFF;

                let r2 = (config.color >> 16) & 0xFF;
                let g2 = (config.color >> 8) & 0xFF;
                let b2 = config.color & 0xFF;

                let r_new = (r1 + r2).min(255);
                let g_new = (g1 + g2).min(255);
                let b_new = (b1 + b2).min(255);

                let updated = 0xFF_00_00_00 | (r_new << 16) | (g_new << 8) | b_new;
                fb.set_pixel(px, py, updated);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clifford_fade() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF_AA_BB_CC);
        CliffordAttractor::fade_framebuffer(&mut fb, 10);
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p, 0xFF_A0_B1_C2);
    }

    #[test]
    fn test_clifford_render() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut attr = CliffordAttractor::default();
        let mut config = CliffordConfig::default();
        config.iters = 100;
        config.color = 0x00_10_20_30;

        attr.render(&mut fb, &config);

        let mut has_color = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_color = true;
                break;
            }
        }

        assert!(has_color);
    }
}
