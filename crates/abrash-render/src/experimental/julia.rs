//! Fractal Generators.
//!
//! Renders mathematical fractals like the Julia set.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Renders a Julia set into the given framebuffer.
pub fn render_julia(
    fb: &mut Framebuffer,
    center_x: f64,
    center_y: f64,
    zoom: f64,
    c_re: f64,
    c_im: f64,
    max_iter: u32,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let w_f64 = width as f64;
    let h_f64 = height as f64;
    let aspect_ratio = w_f64 / h_f64;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f64 = y as f64;
        let p_im = center_y + (y_f64 / h_f64 - 0.5) * zoom;

        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let x_f64 = x as f64;
            let p_re = center_x + (x_f64 / w_f64 - 0.5) * zoom * aspect_ratio;

            let mut z_re = p_re;
            let mut z_im = p_im;
            let mut iter = 0;

            while z_re * z_re + z_im * z_im <= 4.0 && iter < max_iter {
                let z_re_new = z_re * z_re - z_im * z_im + c_re;
                z_im = 2.0 * z_re * z_im + c_im;
                z_re = z_re_new;
                iter += 1;
            }

            // Map iteration count to color
            if iter == max_iter {
                *pixel = 0xFF_00_00_00; // Black for points in the set
            } else {
                let t = iter as f32 / max_iter as f32;
                // Simple color palette
                let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u32;
                let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u32;
                let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u32;
                *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_julia_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        render_julia(&mut fb, 0.0, 0.0, 1.0, -0.7, 0.27015, 100);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_render_julia_standard() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_00_00_00);

        render_julia(&mut fb, 0.0, 0.0, 1.0, -0.7, 0.27015, 10);

        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Framebuffer should not be entirely black after rendering"
        );
    }
}
