use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Generates a Chladni plate resonance pattern on the given framebuffer.
pub fn generate_chladni_pattern(fb: &mut Framebuffer, m: f32, n: f32, time: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let fw = width as f32;
    let fh = height as f32;

    // Pre-calculate trigonometric functions into 1D arrays
    let mut cos_m_x = vec![0.0; width];
    let mut cos_n_x = vec![0.0; width];
    for x in 0..width {
        let nx = (x as f32) / fw * 2.0 - 1.0;
        cos_m_x[x] = (m * PI * nx + time).cos();
        cos_n_x[x] = (n * PI * nx + time).cos();
    }

    let mut cos_m_y = vec![0.0; height];
    let mut cos_n_y = vec![0.0; height];
    for y in 0..height {
        let ny = (y as f32) / fh * 2.0 - 1.0;
        cos_m_y[y] = (m * PI * ny + time).cos();
        cos_n_y[y] = (n * PI * ny + time).cos();
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let cy_m = cos_m_y[y];
        let cy_n = cos_n_y[y];

        for (x, pixel) in row.iter_mut().enumerate() {
            let cx_m = cos_m_x[x];
            let cx_n = cos_n_x[x];

            // Chladni equation
            let term1 = cx_n * cy_m;
            let term2 = cx_m * cy_n;

            let val = (term1 - term2).abs();

            // Threshold for nodal lines
            *pixel = if val < 0.1 {
                0xFF_FF_FF_FF
            } else {
                0xFF_00_00_00
            };
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chladni_plate_generation() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);
        generate_chladni_pattern(&mut fb, 1.0, 2.0, 0.0);

        // Assert that the framebuffer was modified
        let has_content = fb.as_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(has_content, "Chladni generator did not modify the framebuffer!");
    }
}
