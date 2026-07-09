//! Strange Attractor generation.
//!
//! Renders mathematical chaotic attractors like the Clifford attractor
//! directly onto a framebuffer by accumulating density.

use crate::framebuffer::Framebuffer;

/// Parameters for the Clifford Attractor
#[derive(Clone, Copy, Debug)]
pub struct CliffordParams {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

/// Renders a Clifford Attractor onto the given framebuffer.
pub fn render_clifford(
    fb: &mut Framebuffer,
    params: &CliffordParams,
    iterations: u32,
    scale: f64,
    color: u32,
) {
    let width = f64::from(fb.width());
    let height = f64::from(fb.height());
    let hw = width / 2.0;
    let hh = height / 2.0;

    let mut x = 0.0;
    let mut y = 0.0;

    let w_usize = fb.width() as usize;
    let h_usize = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    let r2 = ((color >> 16) & 0xFF) as u32;
    let g2 = ((color >> 8) & 0xFF) as u32;
    let b2 = (color & 0xFF) as u32;

    for _ in 0..iterations {
        let nx = (params.a * y).sin() + params.c * (params.a * x).cos();
        let ny = (params.b * x).sin() + params.d * (params.b * y).cos();
        x = nx;
        y = ny;

        let px = (x * scale + hw) as i32;
        let py = (y * scale + hh) as i32;

        if px >= 0 && px < w_usize as i32 && py >= 0 && py < h_usize as i32 {
            let idx = (py as usize) * w_usize + (px as usize);
            let current = pixels[idx];

            let r1 = ((current >> 16) & 0xFF) as u32;
            let g1 = ((current >> 8) & 0xFF) as u32;
            let b1 = (current & 0xFF) as u32;

            // Add tiny bit of color to accumulate density
            let r = (r1 + (r2 / 10)).min(255);
            let g = (g1 + (g2 / 10)).min(255);
            let b = (b1 + (b2 / 10)).min(255);

            pixels[idx] = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_clifford() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);
        let params = CliffordParams {
            a: 1.5,
            b: -1.8,
            c: 1.6,
            d: 0.9,
        };
        render_clifford(&mut fb, &params, 1000, 20.0, 0xFF_FF_FF_FF);

        let mut has_color = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_color = true;
                break;
            }
        }
        assert!(has_color, "Attractor should draw something");
    }
}
