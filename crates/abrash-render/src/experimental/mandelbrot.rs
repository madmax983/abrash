use abrash_core::framebuffer::Framebuffer;

use rayon::prelude::*;

/// Renders a Mandelbrot set into the given framebuffer.
///
/// The view is defined by `center_x`, `center_y`, and `zoom`.
/// `max_iter` defines the maximum number of iterations for the escape time algorithm.
pub fn render_mandelbrot(
    fb: &mut Framebuffer,
    center_x: f64,
    center_y: f64,
    zoom: f64,
    max_iter: u32,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let aspect_ratio = width as f64 / height as f64;
    let inv_width = 1.0 / width as f64;
    let inv_height = 1.0 / height as f64;

    fb.as_mut_slice()
        .par_chunks_exact_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            let ny = (y as f64 * inv_height - 0.5) * 2.0 / zoom + center_y;

            for (x, pixel) in row.iter_mut().enumerate() {
                let nx = (x as f64 * inv_width - 0.5) * 2.0 * aspect_ratio / zoom + center_x;

                let mut zx = 0.0;
                let mut zy = 0.0;
                let mut iter = 0;
                let mut zx2 = 0.0;
                let mut zy2 = 0.0;

                while zx2 + zy2 <= 4.0 && iter < max_iter {
                    zy = 2.0 * zx * zy + ny;
                    zx = zx2 - zy2 + nx;
                    zx2 = zx * zx;
                    zy2 = zy * zy;
                    iter += 1;
                }

                // Map iterations to a color
                let color = if iter == max_iter {
                    0xFF00_0000 // Black for points inside the set
                } else {
                    // Continuous escape time equation for smooth shading
                    // nu = log2(log2(|z|))
                    let log_z = (zx2 + zy2).ln() * 0.5;
                    let nu = (log_z / 2.0f64.ln()).ln() / 2.0f64.ln();
                    let t = (iter as f64 + 1.0 - nu) / max_iter as f64;

                    let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u32;
                    let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u32;
                    let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u32;
                    0xFF00_0000 | (r << 16) | (g << 8) | b
                };

                *pixel = color;
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_rendering() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000); // Clear to black

        // Render Mandelbrot
        render_mandelbrot(&mut fb, -0.5, 0.0, 1.0, 100);

        // Check that at least one pixel is NOT black (i.e. something was rendered)
        let mut has_color = false;
        for i in 0..100 * 100 {
            if fb.as_slice()[i] != 0xFF00_0000 {
                has_color = true;
                break;
            }
        }

        assert!(
            has_color,
            "Mandelbrot rendering did not produce any output."
        );
    }
}
