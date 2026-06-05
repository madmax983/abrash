//! Thread Art Generator Filter
//!
//! A procedural screen-space effect that simulates string art. It evaluates a target
//! image's luminance and iteratively draws lines (string) between a set of pins arranged
//! around the perimeter to recreate the image using only overlapping lines.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use std::f32::consts::PI;

/// Configuration for the Thread Art effect.
#[derive(Debug, Clone, Copy)]
pub struct ThreadArtConfig {
    /// Number of pins arranged in a circle.
    pub num_pins: usize,
    /// Number of lines (strings) to draw.
    pub max_lines: usize,
    /// Color of the thread (usually black: `0xFF_000000`).
    pub thread_color: u32,
    /// Background color (usually white: `0xFF_FFFFFF`).
    pub bg_color: u32,
    /// Line opacity (alpha). 0.0 to 1.0. Lower values require more lines.
    pub line_weight: f32,
    /// Number of lines to draw per frame in interactive mode.
    pub lines_per_frame: usize,
}

impl Default for ThreadArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 256,
            max_lines: 2000,
            thread_color: 0xFF_000000,
            bg_color: 0xFF_FFFFFF,
            line_weight: 0.1,
            lines_per_frame: 10,
        }
    }
}

/// A structure to hold the state of the thread art generator.
pub struct ThreadArt {
    pub config: ThreadArtConfig,
    pins: Vec<Vec2>,
    current_pin: usize,
    line_count: usize,
    /// A grayscale error map: 0 is target white, 255 is target black
    error_map: Vec<u8>,
    width: usize,
    height: usize,
    initialized: bool,
    /// The lines generated so far (`start_pin`, `end_pin`)
    pub lines: Vec<(usize, usize)>,
}

impl Default for ThreadArt {
    fn default() -> Self {
        Self::new(ThreadArtConfig::default())
    }
}

impl ThreadArt {
    #[must_use]
    pub const fn new(config: ThreadArtConfig) -> Self {
        Self {
            config,
            pins: Vec::new(),
            current_pin: 0,
            line_count: 0,
            error_map: Vec::new(),
            width: 0,
            height: 0,
            initialized: false,
            lines: Vec::new(),
        }
    }

    /// Iteratively draws strings over the target framebuffer (which is assumed to contain the target image on first call)
    pub fn update_and_render(&mut self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        if !self.initialized || self.width != width || self.height != height {
            self.init(fb);
            // Clear FB to background color as we will now only draw strings
            fb.clear(self.config.bg_color);
            // Redraw existing lines if we re-initialized (e.g. resize)
            let existing_lines = self.lines.clone();
            self.lines.clear();
            for &(start, end) in &existing_lines {
                self.draw_line_on_fb(fb, start, end);
                self.lines.push((start, end));
            }
        }

        if self.line_count >= self.config.max_lines {
            return;
        }

        for _ in 0..self.config.lines_per_frame {
            if self.line_count >= self.config.max_lines {
                break;
            }

            let best_next_pin = self.find_best_next_pin();

            // Draw line on framebuffer
            self.draw_line_on_fb(fb, self.current_pin, best_next_pin);

            // Draw line on error map (subtract darkness)
            self.draw_line_on_error_map(self.current_pin, best_next_pin);

            self.lines.push((self.current_pin, best_next_pin));
            self.current_pin = best_next_pin;
            self.line_count += 1;
        }
    }

    fn init(&mut self, fb: &Framebuffer) {
        self.width = fb.width() as usize;
        self.height = fb.height() as usize;
        let w_f32 = self.width as f32;
        let h_f32 = self.height as f32;
        let cx = w_f32 / 2.0;
        let cy = h_f32 / 2.0;
        let radius = (w_f32.min(h_f32) / 2.0) - 2.0;

        // Generate pins
        self.pins.clear();
        for i in 0..self.config.num_pins {
            let angle = i as f32 * 2.0 * PI / self.config.num_pins as f32;
            let x = cx + angle.cos() * radius;
            let y = cy + angle.sin() * radius;
            self.pins.push(Vec2::new(x, y));
        }

        // Generate error map from framebuffer (luminance)
        // We want to approximate the dark areas. So darker = higher error value (need more thread).
        self.error_map = vec![0; self.width * self.height];
        let pixels = fb.as_slice();
        for y in 0..self.height {
            for x in 0..self.width {
                let color = pixels[y * self.width + x];
                let r = ((color >> 16) & 0xFF) as u32;
                let g = ((color >> 8) & 0xFF) as u32;
                let b = (color & 0xFF) as u32;
                // Grayscale luminance
                let lum = (r * 77 + g * 150 + b * 29) >> 8;
                // Invert so dark == high error
                let darkness = 255 - lum;
                self.error_map[y * self.width + x] = darkness as u8;
            }
        }

        self.current_pin = 0;
        self.initialized = true;
    }

    fn find_best_next_pin(&self) -> usize {
        let mut best_pin = 0;
        let mut max_score = -1.0;

        let p0 = self.pins[self.current_pin];

        for i in 1..self.config.num_pins {
            let next_pin = (self.current_pin + i) % self.config.num_pins;

            // Skip nearby pins to avoid drawing polygon edges
            let pin_dist = (self.current_pin as i32 - next_pin as i32).abs();
            let dist = pin_dist.min(self.config.num_pins as i32 - pin_dist);
            if dist < 10 {
                continue;
            }

            let p1 = self.pins[next_pin];
            let score = self.evaluate_line(p0, p1);

            if score > max_score {
                max_score = score;
                best_pin = next_pin;
            }
        }

        best_pin
    }

    fn evaluate_line(&self, p0: Vec2, p1: Vec2) -> f32 {
        let mut score = 0.0;
        let mut count = 0;

        Self::rasterize_line(p0, p1, |x, y| {
            if x < self.width && y < self.height {
                score += f32::from(self.error_map[y * self.width + x]);
                count += 1;
            }
        });

        if count > 0 {
            // Penalize very short lines
            let penalty = if count < 20 { 0.5 } else { 1.0 };
            (score / count as f32) * penalty
        } else {
            0.0
        }
    }

    fn draw_line_on_error_map(&mut self, pin0: usize, pin1: usize) {
        let p0 = self.pins[pin0];
        let p1 = self.pins[pin1];

        let subtraction = (255.0 * self.config.line_weight) as u8;

        let width = self.width;
        let height = self.height;
        let mut temp_map = std::mem::take(&mut self.error_map);

        Self::rasterize_line(p0, p1, |x, y| {
            if x < width && y < height {
                let idx = y * width + x;
                temp_map[idx] = temp_map[idx].saturating_sub(subtraction);
            }
        });

        self.error_map = temp_map;
    }

    fn draw_line_on_fb(&self, fb: &mut Framebuffer, pin0: usize, pin1: usize) {
        let p0 = self.pins[pin0];
        let p1 = self.pins[pin1];
        let thread_color = self.config.thread_color;
        let alpha = (self.config.line_weight * 255.0).clamp(0.0, 255.0) as u32;

        let tr = (thread_color >> 16) & 0xFF;
        let tg = (thread_color >> 8) & 0xFF;
        let tb = thread_color & 0xFF;

        Self::rasterize_line(p0, p1, |x, y| {
            if let Some(bg_color) = fb.get_pixel(x as i32, y as i32) {
                let br = (bg_color >> 16) & 0xFF;
                let bg = (bg_color >> 8) & 0xFF;
                let bb = bg_color & 0xFF;

                let inv_alpha = 255 - alpha;
                let r = (tr * alpha + br * inv_alpha) / 255;
                let g = (tg * alpha + bg * inv_alpha) / 255;
                let b = (tb * alpha + bb * inv_alpha) / 255;

                let out_color = 0xFF_000000 | (r << 16) | (g << 8) | b;
                fb.set_pixel(x as i32, y as i32, out_color);
            }
        });
    }

    fn rasterize_line<F>(p0: Vec2, p1: Vec2, mut plot: F)
    where
        F: FnMut(usize, usize),
    {
        let mut x0 = p0.x as i32;
        let mut y0 = p0.y as i32;
        let x1 = p1.x as i32;
        let y1 = p1.y as i32;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                plot(x0 as usize, y0 as usize);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_art_generation() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Create a simple dark circle in the middle
        fb.clear(0xFF_FFFFFF);
        for y in 30..70 {
            for x in 30..70 {
                if (x - 50) * (x - 50) + (y - 50) * (y - 50) < 400 {
                    fb.set_pixel(x, y, 0xFF_000000);
                }
            }
        }

        let config = ThreadArtConfig {
            num_pins: 100,
            max_lines: 50,
            lines_per_frame: 50,
            ..Default::default()
        };

        let mut art = ThreadArt::new(config);

        // This will initialize and draw all 50 lines in one go
        art.update_and_render(&mut fb);

        assert!(art.initialized);
        assert_eq!(art.line_count, 50);
        assert_eq!(art.lines.len(), 50);

        // Verify some pixels are dark now (string drawn)
        let mut has_dark_pixels = false;
        for &p in fb.as_slice() {
            if p != 0xFF_FFFFFF {
                has_dark_pixels = true;
                break;
            }
        }
        assert!(has_dark_pixels, "No strings were drawn on the framebuffer");
    }
}
