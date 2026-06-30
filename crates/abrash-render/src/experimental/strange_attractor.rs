use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn render_clifford_attractor(
    fb: &mut Framebuffer,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    iters: usize,
) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let cx = width / 2;
    let cy = height / 2;
    let scale = (width.min(height) as f32) / 5.0;

    // ⚡ Bolt Optimization: Use a 1D density histogram to avoid per-pixel color packing and atomic contention in the hot loop
    let mut density = vec![0u32; (width * height) as usize];

    let mut x = 0.0f32;
    let mut y = 0.0f32;

    for _ in 0..iters {
        let nx = (a * y).sin() + c * (a * x).cos();
        let ny = (b * x).sin() + d * (b * y).cos();
        x = nx;
        y = ny;

        let px = cx + (x * scale) as i32;
        let py = cy + (y * scale) as i32;

        if px >= 0 && px < width && py >= 0 && py < height {
            let idx = (py * width + px) as usize;
            density[idx] += 1;
        }
    }

    let pixels = fb.as_mut_slice();
    let w_usize = width as usize;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(w_usize).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(w_usize).enumerate();

    row_iter.for_each(|(y, row)| {
        let row_offset = y * w_usize;
        for (x, pixel) in row.iter_mut().enumerate() {
            let count = density[row_offset + x];
            if count > 0 {
                let intensity = (count as f32 * 5.0).min(255.0) as u32;
                *pixel =
                    0xFF_00_00_00 | (intensity << 16) | ((intensity / 2) << 8) | (intensity / 4);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_attractor_draws() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);
        render_clifford_attractor(&mut fb, 1.5, -1.8, 1.6, 2.0, 10000);
        let has_non_black = fb.as_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(has_non_black, "Framebuffer should not be entirely black");
    }
}
