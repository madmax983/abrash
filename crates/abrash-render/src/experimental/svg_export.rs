//! SVG Exporter for Framebuffers
//!
//! This module provides an experimental feature to export a `Framebuffer`
//! as a Scalable Vector Graphics (SVG) file.
//!
//! While exporting a pixel buffer to a vector format might seem counterintuitive,
//! it enables interesting capabilities such as importing rendered frames into
//! vector editors (like Illustrator or Inkscape) for stylized vector manipulation,
//! crisp scaling, or creating mosaic-like vector art where each pixel is a `<rect>`.
//!
//! # Implementation Details
//!
//! To keep the resulting SVG file size manageable, this exporter implements a basic
//! Run-Length Encoding (RLE) per row. Contiguous pixels of the exact same color
//! are merged into a single wider `<rect>` element. This significantly reduces
//! the element count for flat-shaded or low-detail areas.

use abrash_core::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Trait to export structures as an SVG file.
pub trait SvgExporter {
    /// Exports the object to an SVG file at the specified path.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the file cannot be created or written to.
    fn export_svg<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

impl SvgExporter for Framebuffer {
    fn export_svg<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let width = self.width();
        let height = self.height();
        let pixels = self.as_slice();

        let file = File::create(path)?;
        // Use a BufWriter to significantly speed up thousands of small writes.
        let mut writer = BufWriter::new(file);

        // SVG Header
        writeln!(
            writer,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#
        )?;

        // Iterate over rows and apply simple RLE (Run-Length Encoding)
        for y in 0..height {
            let row_start = (y * width) as usize;
            let mut current_color: Option<u32> = None;
            let mut run_start_x = 0;

            for x in 0..width {
                let pixel = pixels[row_start + x as usize];

                let alpha = (pixel >> 24) & 0xFF;

                if alpha == 0 {
                    // It's transparent. Flush the current run if any.
                    if let Some(color) = current_color {
                        write_rect(&mut writer, run_start_x, y, x - run_start_x, color)?;
                        current_color = None;
                    }
                    continue;
                }

                match current_color {
                    Some(color) if color == pixel => {
                        // Continue the run
                    }
                    Some(color) => {
                        // Color changed. Flush the previous run.
                        write_rect(&mut writer, run_start_x, y, x - run_start_x, color)?;
                        // Start new run
                        current_color = Some(pixel);
                        run_start_x = x;
                    }
                    None => {
                        // Start first run
                        current_color = Some(pixel);
                        run_start_x = x;
                    }
                }
            }

            // End of row: flush any remaining run
            if let Some(color) = current_color {
                write_rect(&mut writer, run_start_x, y, width - run_start_x, color)?;
            }
        }

        // SVG Footer
        writeln!(writer, "</svg>")?;

        // Ensure everything is flushed to disk
        writer.flush()?;

        Ok(())
    }
}

/// Helper function to write a single SVG `<rect>` element.
#[inline]
fn write_rect(
    writer: &mut BufWriter<File>,
    x: u32,
    y: u32,
    width: u32,
    color: u32,
) -> io::Result<()> {
    let a = (color >> 24) & 0xFF;
    let r = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = color & 0xFF;

    if a == 255 {
        writeln!(
            writer,
            r#"  <rect x="{x}" y="{y}" width="{width}" height="1" fill="rgb({r},{g},{b})" />"#
        )
    } else {
        // SVG uses opacity from 0.0 to 1.0
        let opacity = a as f32 / 255.0;
        writeln!(
            writer,
            r#"  <rect x="{x}" y="{y}" width="{width}" height="1" fill="rgb({r},{g},{b})" fill-opacity="{opacity:.3}" />"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_svg_export() {
        let mut fb = Framebuffer::new(4, 2).unwrap();

        // Row 0: 2 Red, 2 Green (Testing RLE)
        fb.set_pixel(0, 0, 0xFFFF_0000);
        fb.set_pixel(1, 0, 0xFFFF_0000);
        fb.set_pixel(2, 0, 0xFF00_FF00);
        fb.set_pixel(3, 0, 0xFF00_FF00);

        // Row 1: 1 Blue, 1 Transparent, 2 Semi-transparent Blue
        fb.set_pixel(0, 1, 0xFF00_00FF);
        fb.set_pixel(1, 1, 0x0000_0000); // Fully transparent
        fb.set_pixel(2, 1, 0x8000_00FF);
        fb.set_pixel(3, 1, 0x8000_00FF);

        let path = "test_output.svg";

        // Ensure the file doesn't exist before we test
        let _ = fs::remove_file(path);

        fb.export_svg(path).unwrap();

        // Read and verify the contents
        let contents = fs::read_to_string(path).unwrap();

        // Verify Header
        assert!(contents.contains(r#"viewBox="0 0 4 2""#));

        // Verify Row 0 (RLE combined to width=2)
        assert!(contents.contains(r#"<rect x="0" y="0" width="2" height="1" fill="rgb(255,0,0)""#));
        assert!(contents.contains(r#"<rect x="2" y="0" width="2" height="1" fill="rgb(0,255,0)""#));

        // Verify Row 1
        assert!(contents.contains(r#"<rect x="0" y="1" width="1" height="1" fill="rgb(0,0,255)""#));
        // Transparent pixel should not produce a rect
        // Semi-transparent blue should be combined and have fill-opacity
        assert!(contents.contains(r#"<rect x="2" y="1" width="2" height="1" fill="rgb(0,0,255)" fill-opacity="#));

        // Clean up
        fs::remove_file(path).unwrap();
    }
}
