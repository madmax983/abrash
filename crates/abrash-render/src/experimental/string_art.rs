use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// Applies a String Art (Thread Art) effect to the given framebuffer.
///
/// It approximates the luminance of the input image using a single continuous thread
/// wrapped around a set of pins placed in a circle.
///
/// # Panics
/// Panics if `num_pins` is less than 5.
pub fn apply_string_art(
    fb: &mut Framebuffer,
    num_pins: usize,
    num_lines: usize,
    thread_opacity: f32,
) {
    assert!(num_pins >= 5, "String art requires at least 5 pins");

    let w = fb.width();
    let h = fb.height();
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let radius = (w.min(h) as f32 / 2.0) - 4.0; // 4px padding

    // Convert framebuffer to grayscale error map (0.0 to 1.0, where 1.0 is dark/needs thread)
    let mut error_map = vec![0.0f32; (w * h) as usize];
    for (i, &pixel) in fb.as_slice().iter().enumerate() {
        let r = ((pixel >> 16) & 0xFF) as f32 / 255.0;
        let g = ((pixel >> 8) & 0xFF) as f32 / 255.0;
        let b = (pixel & 0xFF) as f32 / 255.0;
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        error_map[i] = 1.0 - lum;
    }

    // Generate pin positions
    let mut pins = Vec::with_capacity(num_pins);
    for i in 0..num_pins {
        let angle = i as f32 * 2.0 * PI / num_pins as f32;
        let px = cx + angle.cos() * radius;
        let py = cy + angle.sin() * radius;
        pins.push((px, py));
    }

    // Clear fb to white
    fb.as_mut_slice().fill(0xFF_FF_FF_FF);

    let mut current_pin = 0;

    for _ in 0..num_lines {
        let mut best_pin = current_pin;
        let mut best_score = -1.0;
        let mut best_line_pixels = Vec::new();

        for next_pin in 0..num_pins {
            let diff = (next_pin as isize - current_pin as isize).abs();
            let min_dist = (num_pins / 10).max(1) as isize; if diff < min_dist || diff > num_pins as isize - min_dist {
                continue;
            }

            let line_pixels = get_line_pixels(w, h, pins[current_pin], pins[next_pin]);

            let mut score = 0.0;
            for &(x, y) in &line_pixels {
                score += error_map[(y * w as i32 + x) as usize];
            }

            if !line_pixels.is_empty() {
                score /= line_pixels.len() as f32;
            }

            // penalize reusing pins too often or doing straight reversals
            if score > best_score {
                best_score = score;
                best_pin = next_pin;
                best_line_pixels = line_pixels;
            }
        }

        for &(x, y) in &best_line_pixels {
            let idx = (y * w as i32 + x) as usize;
            error_map[idx] = (error_map[idx] - thread_opacity).max(0.0);

            let current_color = fb.as_slice()[idx];
            let r = ((current_color >> 16) & 0xFF) as f32 / 255.0;
            let g = ((current_color >> 8) & 0xFF) as f32 / 255.0;
            let b = (current_color & 0xFF) as f32 / 255.0;

            let new_r = (r - thread_opacity).max(0.0);
            let new_g = (g - thread_opacity).max(0.0);
            let new_b = (b - thread_opacity).max(0.0);

            let r_u8 = (new_r * 255.0) as u32;
            let g_u8 = (new_g * 255.0) as u32;
            let b_u8 = (new_b * 255.0) as u32;

            fb.as_mut_slice()[idx] = 0xFF_00_00_00 | (r_u8 << 16) | (g_u8 << 8) | b_u8;
        }

        current_pin = best_pin;
    }
}

fn get_line_pixels(w: u32, h: u32, p0: (f32, f32), p1: (f32, f32)) -> Vec<(i32, i32)> {
    let mut pixels = Vec::new();
    let mut x0 = p0.0 as i32;
    let mut y0 = p0.1 as i32;
    let x1 = p1.0 as i32;
    let y1 = p1.1 as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            pixels.push((x0, y0));
        }

        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_string_art() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        fb.as_mut_slice().fill(0xFF_00_00_00); // Black image (needs threads)
        apply_string_art(&mut fb, 20, 100, 0.1);

        // Assert that at least some pixels have been drawn (are not white)
        let has_threads = fb.as_slice().iter().any(|&p| p != 0xFF_FF_FF_FF);
        assert!(has_threads);
    }

    #[test]
    #[should_panic(expected = "String art requires at least 5 pins")]
    fn test_string_art_panic() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        apply_string_art(&mut fb, 4, 100, 0.1);
    }
}
