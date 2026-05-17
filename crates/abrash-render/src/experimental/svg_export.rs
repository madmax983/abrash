//! SVG Exporter
//!
//! An experimental feature to export framebuffers as vector graphics (SVG).
//! It utilizes a horizontal Run-Length Encoding (RLE) to drastically reduce
//! the number of generated `<rect>` elements by grouping adjacent identical pixels.

use abrash_core::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Trait to allow exporting framebuffers to SVG format.
pub trait SvgExportable {
    /// Exports the framebuffer to an SVG file using horizontal RLE.
    fn export_svg<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

impl SvgExportable for Framebuffer {
    fn export_svg<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let width = self.width();
        let height = self.height();

        writeln!(
            writer,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
        )?;
        writeln!(
            writer,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
            width, height, width, height
        )?;

        // Background (optional: could fill with the most common color)
        writeln!(
            writer,
            "  <rect width=\"100%\" height=\"100%\" fill=\"#000000\"/>"
        )?;

        if width > 0 && height > 0 {
            for y in 0..height {
                let mut current_color: Option<u32> = None;
                let mut start_x = 0;

                for x in 0..width {
                    let color = self.get_pixel(x as i32, y as i32).unwrap_or(0);
                    // Skip fully transparent pixels
                    let a = (color >> 24) & 0xFF;

                    if a == 0 {
                        if let Some(c) = current_color {
                            write_rect(&mut writer, start_x, y, x - start_x, c)?;
                            current_color = None;
                        }
                        continue;
                    }

                    match current_color {
                        Some(c) if c == color => {
                            // Continue RLE run
                        }
                        Some(c) => {
                            // Color changed, write previous run
                            write_rect(&mut writer, start_x, y, x - start_x, c)?;
                            current_color = Some(color);
                            start_x = x;
                        }
                        None => {
                            // Start new run
                            current_color = Some(color);
                            start_x = x;
                        }
                    }
                }

                // Write final run of the row
                if let Some(c) = current_color {
                    write_rect(&mut writer, start_x, y, width - start_x, c)?;
                }
            }
        }

        writeln!(writer, "</svg>")?;
        Ok(())
    }
}

fn write_rect<W: Write>(writer: &mut W, x: u32, y: u32, w: u32, color: u32) -> io::Result<()> {
    let r = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = color & 0xFF;
    let a = (color >> 24) & 0xFF;

    if a == 255 {
        writeln!(
            writer,
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"1\" fill=\"#{:02x}{:02x}{:02x}\"/>",
            x, y, w, r, g, b
        )
    } else {
        writeln!(
            writer,
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"1\" fill=\"#{:02x}{:02x}{:02x}\" opacity=\"{:.3}\"/>",
            x, y, w, r, g, b, (a as f32) / 255.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;
    use std::fs;

    #[test]
    fn test_svg_export_rle() {
        let mut fb = Framebuffer::new(10, 2).unwrap();

        // Row 0: 5 red pixels, 5 blue pixels -> 2 rects
        for x in 0..5 {
            fb.set_pixel(x, 0, 0xFFFF0000); // Red
        }
        for x in 5..10 {
            fb.set_pixel(x, 0, 0xFF0000FF); // Blue
        }

        // Row 1: 10 green pixels -> 1 rect
        for x in 0..10 {
            fb.set_pixel(x, 1, 0xFF00FF00); // Green
        }

        let path = "test_rle.svg";
        fb.export_svg(path).unwrap();

        let content = fs::read_to_string(path).unwrap();
        let rect_count = content.matches("<rect x=").count();

        // 3 rects total (plus the 1 background rect which doesn't have an x attribute)
        assert_eq!(rect_count, 3);

        fs::remove_file(path).unwrap();
    }
}
