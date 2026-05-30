use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;

pub struct StringArt {
    pub points: usize,
    pub multiplier: f32,
    pub radius: f32,
    pub center: Vec2,
}

impl StringArt {
    pub fn new(points: usize, multiplier: f32, radius: f32, center: Vec2) -> Self {
        Self {
            points,
            multiplier,
            radius,
            center,
        }
    }

    /// Renders the string art pattern onto a framebuffer.
    /// Expects a 32-bit ARGB buffer (0xAARRGGBB).
    pub fn render(&self, buffer: &mut Framebuffer) {
        let n = self.points;
        if n < 2 {
            return;
        }

        let angle_step = std::f32::consts::TAU / (n as f32);

        // Pre-calculate points on the circle
        // ⚡ Bolt: Using fast_sin_cos if available, and avoiding dynamic allocation for points
        // by calculating on the fly if n is very large, but we can stick to pre-calc for now
        // since we just want to avoid re-allocating. Actually, for optimization, let's precalc
        // sin/cos using our fast funcs.
        use abrash_core::math::funcs::fast_sin_cos;
        let mut circle_points = Vec::with_capacity(n);
        for i in 0..n {
            let angle = (i as f32) * angle_step;
            // ⚡ Bolt: Use fast_sin_cos which is slightly less precise but faster
            let (sin_a, cos_a) = fast_sin_cos(angle);
            let px = self.center.x + self.radius * cos_a;
            let py = self.center.y + self.radius * sin_a;
            circle_points.push((px as i32, py as i32));
        }

        let color = 0xFFFFFFFF; // White color

        for i in 0..n {
            // Integer multiplication is faster and exact for integer multipliers
            // If the multiplier is not integer, we fall back to float
            let j = if self.multiplier.fract() == 0.0 {
                // Cast to f64 to avoid overflow for large point counts, and handle negative properly
                let mult = self.multiplier as i64;
                let mut idx = (i as i64 * mult) % (n as i64);
                if idx < 0 {
                    idx += n as i64;
                }
                idx as usize
            } else {
                let mut idx_f = (i as f32 * self.multiplier) % (n as f32);
                if idx_f < 0.0 {
                    idx_f += n as f32;
                }
                idx_f as usize
            };

            let p0 = circle_points[i];
            let p1 = circle_points[j];

            draw_line_2d_fast(buffer, p0.0, p0.1, p1.0, p1.1, color);
        }
    }
}

// ⚡ Bolt: Optimized line drawing algorithm utilizing bounds checks on the whole line,
// and unsafe pixel plotting in the inner loop to elide checks.
fn draw_line_2d_fast(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Quick bounds check - if the whole line is outside on one side, cull it
    if (x0 < 0 && x1 < 0) || (x0 >= width && x1 >= width) ||
       (y0 < 0 && y1 < 0) || (y0 >= height && y1 >= height) {
        return;
    }

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    // Check if the entire bounding box of the line is on screen
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    if min_x >= 0 && max_x < width && min_y >= 0 && max_y < height {
        // Safe fast path - no per-pixel bounds checking required
        let width_usize = width as usize;
        let slice = fb.as_mut_slice();
        loop {
            // SAFETY: We proved the entire line is on screen above.
            unsafe {
                *slice.get_unchecked_mut((y0 as usize) * width_usize + (x0 as usize)) = color;
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
    } else {
        // Slow path - clip per pixel
        loop {
            fb.set_pixel(x0, y0, color);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_points_generation() {
        let art = StringArt::new(10, 2.0, 100.0, Vec2::new(100.0, 100.0));
        let mut buffer = Framebuffer::new(200, 200).unwrap();
        art.render(&mut buffer);

        // A blank framebuffer is initialized to 0xFF00_0000 (opaque black).
        let pixels = buffer.as_slice();

        let mut drawn_pixels = 0;
        for &pixel in pixels {
            // Only count pixels that are NOT the background color
            if pixel != 0xFF00_0000 {
                drawn_pixels += 1;
            }
        }

        // Ensure at least some pixels were drawn
        assert!(drawn_pixels > 0, "No pixels were drawn by StringArt render");
    }
}
