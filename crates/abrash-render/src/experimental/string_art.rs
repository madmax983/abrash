use abrash_core::framebuffer::Framebuffer;

pub struct StringArtConfig {
    pub num_pegs: usize,
    pub num_lines: usize,
    pub thread_color: u32,
    pub bg_color: u32,
    pub thread_alpha: f32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pegs: 256,
            num_lines: 1000,
            thread_color: 0xFF_000000,
            bg_color: 0xFF_FFFFFF,
            thread_alpha: 0.1, // 10% opacity
        }
    }
}

use abrash_core::math::Vec2;

pub fn render_string_art(source: &Framebuffer, target: &mut Framebuffer, config: &StringArtConfig) {
    let width = target.width();
    let height = target.height();

    if width == 0 || height == 0 || width != source.width() || height != source.height() {
        return;
    }

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = f32::min(center_x, center_y) - 2.0; // Small padding

    // 1. Pre-calculate peg positions around the circle
    let mut pegs = Vec::with_capacity(config.num_pegs);
    for i in 0..config.num_pegs {
        let angle = (i as f32 / config.num_pegs as f32) * std::f32::consts::TAU;
        pegs.push(Vec2::new(
            center_x + radius * angle.cos(),
            center_y + radius * angle.sin(),
        ));
    }

    // 2. Pre-calculate a darkness map from the source image.
    // 0.0 means white (no thread needed), 1.0 means black (needs thread).
    let mut darkness_map = vec![0.0f32; (width * height) as usize];
    for (i, &pixel) in source.as_slice().iter().enumerate() {
        let r = ((pixel >> 16) & 0xFF) as f32;
        let g = ((pixel >> 8) & 0xFF) as f32;
        let b = (pixel & 0xFF) as f32;
        // Simple luminance calculation
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
        darkness_map[i] = 1.0 - (luminance / 255.0);
    }

    // 3. Weave the threads!
    let mut current_peg = 0;
    let mut rng = abrash_core::utils::XorShift32::new(0xDEAD_BEEF); // simple rng to break ties if needed

    // Draw the background once
    target.clear(config.bg_color);

    let thread_r = ((config.thread_color >> 16) & 0xFF) as f32;
    let thread_g = ((config.thread_color >> 8) & 0xFF) as f32;
    let thread_b = (config.thread_color & 0xFF) as f32;
    let alpha = config.thread_alpha.clamp(0.0, 1.0);
    let inv_alpha = 1.0 - alpha;

    for _ in 0..config.num_lines {
        let mut best_peg = current_peg;
        let mut best_score = -1.0;
        let p1 = pegs[current_peg];

        // Find the peg that results in drawing a line over the darkest path
        for i in 1..config.num_pegs {
            let next_peg = (current_peg + i) % config.num_pegs;
            let p2 = pegs[next_peg];

            // Avoid drawing to immediate neighbors
            if i < 3 || i > config.num_pegs - 3 {
                continue;
            }

            // Sample the darkness map along the line using Bresenham-like approach
            let score = score_line(
                &darkness_map,
                width as i32,
                p1.x as i32,
                p1.y as i32,
                p2.x as i32,
                p2.y as i32,
            );

            if score > best_score {
                best_score = score;
                best_peg = next_peg;
            }
        }

        // If we can't find a good peg, just pick a random one across the circle
        if best_score <= 0.0 {
            best_peg = (current_peg + config.num_pegs / 2 + (rng.next_u32() % 10) as usize)
                % config.num_pegs;
        }

        // Draw the line on the target framebuffer and subtract its darkness from our map
        draw_and_subtract_line(
            target,
            &mut darkness_map,
            width as i32,
            height as i32,
            p1.x as i32,
            p1.y as i32,
            pegs[best_peg].x as i32,
            pegs[best_peg].y as i32,
            thread_r,
            thread_g,
            thread_b,
            alpha,
            inv_alpha,
        );

        current_peg = best_peg;
    }
}

/// Evaluates how "dark" the source image is along a given line.
fn score_line(darkness_map: &[f32], width: i32, x0: i32, y0: i32, x1: i32, y1: i32) -> f32 {
    let mut score = 0.0;
    let mut count = 0;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    // y must be checked too to avoid negative y values wrapping around to large usize indices
    let height = (darkness_map.len() as i32) / width;

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            let idx = (y * width + x) as usize;
            if idx < darkness_map.len() {
                score += darkness_map[idx];
                count += 1;
            }
        }

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }

    if count > 0 { score / count as f32 } else { 0.0 }
}

/// Draws an aliased line to the framebuffer while subtracting its weight from the darkness map.
#[allow(clippy::too_many_arguments)]
fn draw_and_subtract_line(
    target: &mut Framebuffer,
    darkness_map: &mut [f32],
    width: i32,
    height: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    r: f32,
    g: f32,
    b: f32,
    alpha: f32,
    inv_alpha: f32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    // A rough deduction factor to simulate the thread covering some darkness
    let deduction = 0.1;

    let pixels = target.as_mut_slice();

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            let idx = (y * width + x) as usize;
            if idx < darkness_map.len() {
                // Deduct from the map so we don't keep drawing over the same dark areas
                darkness_map[idx] = (darkness_map[idx] - deduction).max(0.0);

                // Blend pixel
                let current = pixels[idx];
                let bg_r = ((current >> 16) & 0xFF) as f32;
                let bg_g = ((current >> 8) & 0xFF) as f32;
                let bg_b = (current & 0xFF) as f32;

                let new_r = (r * alpha + bg_r * inv_alpha) as u32;
                let new_g = (g * alpha + bg_g * inv_alpha) as u32;
                let new_b = (b * alpha + bg_b * inv_alpha) as u32;

                pixels[idx] = 0xFF_000000 | (new_r << 16) | (new_g << 8) | new_b;
            }
        }

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_modifies_target() {
        let mut source = Framebuffer::new(100, 100).unwrap();
        // create a source image (black circle in center)
        for y in 25..75 {
            for x in 25..75 {
                source.set_pixel(x, y, 0xFF_000000);
            }
        }

        let mut target = Framebuffer::new(100, 100).unwrap();
        target.clear(0xFF_FFFFFF); // White background

        let config = StringArtConfig {
            num_pegs: 32,
            num_lines: 50,
            thread_color: 0xFF_000000,
            bg_color: 0xFF_FFFFFF,
            thread_alpha: 1.0,
        };

        render_string_art(&source, &mut target, &config);

        // Target should not be completely white anymore
        let mut has_non_white = false;
        for &pixel in target.as_slice() {
            if pixel != 0xFF_FFFFFF {
                has_non_white = true;
                break;
            }
        }
        assert!(
            has_non_white,
            "Target framebuffer should contain drawn lines"
        );
    }
}
