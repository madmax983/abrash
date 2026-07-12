//! CSS Art Exporter.
//!
//! Exports a `Framebuffer` into a single-div HTML/CSS file using the `box-shadow` technique.
//! This turns any rendered 3D scene into a completely valid, dependency-free CSS art piece.

use abrash_core::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Exports the framebuffer to a single HTML file containing a single `<div>`
/// that renders the entire image using a massive CSS `box-shadow`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to export.
/// * `path` - The output path for the HTML file.
/// * `pixel_size` - The CSS size (in pixels) of each logical pixel.
///
/// # Errors
///
/// Returns an `io::Error` if the output file cannot be created or written to.
pub fn export_to_css_art<P: AsRef<Path>>(
    fb: &Framebuffer,
    path: P,
    pixel_size: u32,
) -> io::Result<()> {
    let mut file = File::create(path)?;

    let width = fb.width();
    let height = fb.height();

    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html>")?;
    writeln!(file, "<head>")?;
    writeln!(file, "<style>")?;
    writeln!(
        file,
        "  body {{ background: #111; display: flex; justify-content: center; align-items: center; min-height: 100vh; margin: 0; }}"
    )?;
    writeln!(file, "  .css-art {{")?;
    writeln!(file, "    width: {pixel_size}px;")?;
    writeln!(file, "    height: {pixel_size}px;")?;

    let mut shadows = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = fb.get_pixel(x as i32, y as i32).unwrap_or(0);
            let a = (pixel >> 24) & 0xFF;
            if a == 0 {
                continue;
            } // Skip fully transparent pixels

            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            let alpha = a as f32 / 255.0;
            let shadow_x = x * pixel_size;
            let shadow_y = y * pixel_size;

            let shadow = format!("{shadow_x}px {shadow_y}px rgba({r},{g},{b},{alpha:.3})");
            shadows.push(shadow);
        }
    }

    if !shadows.is_empty() {
        writeln!(file, "    box-shadow:")?;
        let joined_shadows = shadows.join(",\n      ");
        writeln!(file, "      {joined_shadows};")?;
    }

    // We need to offset the entire div back because the box-shadows are cast *from* the top-left pixel.
    let margin_r = width * pixel_size;
    let margin_b = height * pixel_size;
    writeln!(file, "    margin-right: {margin_r}px;")?;
    writeln!(file, "    margin-bottom: {margin_b}px;")?;

    writeln!(file, "  }}")?;
    writeln!(file, "</style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;
    writeln!(file, "  <div class=\"css-art\"></div>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_css_art() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000);
        fb.set_pixel(1, 1, 0xFF00_FF00);

        let path = std::env::temp_dir().join("test_css_art.html");
        assert!(export_to_css_art(&fb, &path, 10).is_ok());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("box-shadow:"));
        assert!(content.contains("rgba(255,0,0,1.000)"));
        assert!(content.contains("rgba(0,255,0,1.000)"));
    }
}
