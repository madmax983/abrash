#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn render_mandelbrot(buffer: &mut [u32], width: usize, height: usize) {
    if buffer.is_empty() || width == 0 || height == 0 {
        return;
    }

    let max_iter = 100;
    let min_x = -2.0;
    let max_x = 1.0;
    let min_y = -1.5;
    let max_y = 1.5;

    let dx = (max_x - min_x) / width as f32;
    let dy = (max_y - min_y) / height as f32;

    #[cfg(feature = "parallel")]
    let iter = buffer.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = buffer.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        let cy = min_y + y as f32 * dy;

        for x in 0..width {
            let cx = min_x + x as f32 * dx;

            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iter_count = 0;

            let mut zx2 = 0.0;
            let mut zy2 = 0.0;

            while zx2 + zy2 <= 4.0 && iter_count < max_iter {
                zy = 2.0 * zx * zy + cy;
                zx = zx2 - zy2 + cx;
                zx2 = zx * zx;
                zy2 = zy * zy;
                iter_count += 1;
            }

            let color = if iter_count == max_iter {
                0xFF_00_00_00 // Black for points inside the set
            } else {
                let intensity = (iter_count as f32 / max_iter as f32 * 255.0) as u32;
                0xFF_00_00_00 | (intensity << 16) | (intensity << 8) | intensity
            };

            row[x] = color;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_basic_render() {
        let width = 100;
        let height = 100;
        let mut buffer = vec![0; width * height];

        render_mandelbrot(&mut buffer, width, height);

        let center_idx = (height / 2) * width + (width / 2);
        assert_eq!(buffer[center_idx], 0xFF_00_00_00);

        let edge_idx = 0;
        assert_ne!(buffer[edge_idx], 0xFF_00_00_00);
    }
}
