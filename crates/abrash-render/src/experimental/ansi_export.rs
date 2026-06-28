//! ANSI Terminal Exporter
//!
//! Exports a Framebuffer to a string containing ANSI escape codes, allowing
//! images to be printed directly to the terminal using the half-block character (▀).

use abrash_core::framebuffer::Framebuffer;
use std::fmt::Write;

/// Converts a Framebuffer into an ANSI truecolor string.
///
/// Uses the half-block character (▀) to represent two vertical pixels at once:
/// the foreground color corresponds to the top pixel, and the background color
/// corresponds to the bottom pixel.
#[must_use]
pub fn export_to_ansi(fb: &Framebuffer) -> String {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return String::new();
    }

    // Pre-allocate assuming ~40 bytes per pixel pair (escape codes + char)
    let mut out = String::with_capacity((width * height / 2) * 40);

    let mut current_fg = None;
    let mut current_bg = None;

    for y in (0..height).step_by(2) {
        for x in 0..width {
            let top_pixel = fb.get_pixel(x as i32, y as i32).unwrap_or(0xFF00_0000);
            let bottom_pixel = if y + 1 < height {
                fb.get_pixel(x as i32, (y + 1) as i32)
                    .unwrap_or(0xFF00_0000)
            } else {
                0xFF00_0000 // Pad with black if odd height
            };

            let fg_r = (top_pixel >> 16) & 0xFF;
            let fg_g = (top_pixel >> 8) & 0xFF;
            let fg_b = top_pixel & 0xFF;

            let bg_r = (bottom_pixel >> 16) & 0xFF;
            let bg_g = (bottom_pixel >> 8) & 0xFF;
            let bg_b = bottom_pixel & 0xFF;

            let fg = (fg_r, fg_g, fg_b);
            let bg = (bg_r, bg_g, bg_b);

            if current_fg != Some(fg) {
                let _ = write!(&mut out, "\x1B[38;2;{fg_r};{fg_g};{fg_b}m");
                current_fg = Some(fg);
            }

            if current_bg != Some(bg) {
                let _ = write!(&mut out, "\x1B[48;2;{bg_r};{bg_g};{bg_b}m");
                current_bg = Some(bg);
            }

            out.push('▀');
        }

        // Reset colors at the end of each row and add newline
        out.push_str("\x1B[0m\n");
        current_fg = None;
        current_bg = None;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_to_ansi() {
        let mut fb = Framebuffer::new(1, 2).unwrap();
        // Top pixel: Red
        fb.set_pixel(0, 0, 0xFFFF0000);
        // Bottom pixel: Blue
        fb.set_pixel(0, 1, 0xFF0000FF);

        let result = export_to_ansi(&fb);
        // Expected: Esc[38;2;255;0;0m Esc[48;2;0;0;255m ▀ Esc[0m \n
        let expected = "\x1B[38;2;255;0;0m\x1B[48;2;0;0;255m▀\x1B[0m\n";
        assert_eq!(result, expected);
    }
}
