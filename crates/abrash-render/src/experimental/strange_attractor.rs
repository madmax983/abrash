//! Strange Attractor Generator.
//!
//! Renders Clifford strange attractors.

use crate::framebuffer::Framebuffer;

/// Renders a Clifford attractor into the given framebuffer.
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
    if width == 0 || height == 0 {
        return;
    }

    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;

    let w_f32 = width as f32;
    let h_f32 = height as f32;
    let scale = (w_f32.min(h_f32) * 0.2) as f32;
    let cx = w_f32 * 0.5;
    let cy = h_f32 * 0.5;

    for _ in 0..iters {
        let x_new = (a * y).sin() + c * (a * x).cos();
        let y_new = (b * x).sin() + d * (b * y).cos();

        x = x_new;
        y = y_new;

        let px = (x * scale + cx) as i32;
        let py = (y * scale + cy) as i32;

        if px >= 0 && px < width && py >= 0 && py < height {
            let idx = (py * width + px) as usize;
            let current = fb.as_slice()[idx];
            let current_r = (current >> 16) & 0xFF;
            let current_g = (current >> 8) & 0xFF;
            let current_b = current & 0xFF;

            let new_r = (current_r + 2).min(255);
            let new_g = (current_g + 5).min(255);
            let new_b = (current_b + 10).min(255);

            fb.set_pixel(
                px,
                py,
                0xFF00_0000u32 | (new_r << 16) | (new_g << 8) | new_b,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_clifford_attractor() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        render_clifford_attractor(&mut fb, -1.4, 1.6, 1.0, 0.7, 1000);
        let mut has_color = false;
        for &p in fb.as_slice() {
            if p != 0 {
                has_color = true;
                break;
            }
        }
        assert!(has_color, "Attractor did not render any pixels");
    }
}
