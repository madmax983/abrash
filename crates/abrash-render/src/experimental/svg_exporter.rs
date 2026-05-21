//! SVG Exporter
//!
//! A simple utility to export a Framebuffer to an SVG file.

use abrash_core::framebuffer::Framebuffer;
use std::fmt::Write;

/// Defines the shape used to represent each pixel in the exported SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgShape {
    /// Export pixels as continuous rectangles.
    Rect,
    /// Export pixels as isolated circles (dots) for a stylized look.
    Circle,
}

/// Configuration for the SVG export.
#[derive(Debug, Clone)]
pub struct SvgExportConfig {
    /// The shape to use for each exported pixel.
    pub shape: SvgShape,
    /// The scale factor. A scale of 1 means 1 pixel = 1 unit in SVG space.
    pub scale: usize,
}

impl Default for SvgExportConfig {
    fn default() -> Self {
        Self {
            shape: SvgShape::Rect,
            scale: 1,
        }
    }
}

/// Exports the given Framebuffer to an SVG string based on the configuration.
#[must_use]
pub fn export_to_svg(fb: &Framebuffer, config: &SvgExportConfig) -> String {
    let width = fb.width();
    let height = fb.height();
    let scale = config.scale;

    let svg_width = width as usize * scale;
    let svg_height = height as usize * scale;

    let mut svg = String::with_capacity((width * height).min(10000) as usize * 60);

    let _ = writeln!(
        &mut svg,
        "<svg width=\"{svg_width}\" height=\"{svg_height}\" xmlns=\"http://www.w3.org/2000/svg\">"
    );

    let pixels = fb.as_slice();

    for y in 0..height {
        let y_scaled = y as usize * scale;

        let mut x = 0;
        while x < width {
            let idx = (y * width + x) as usize;
            let p = pixels[idx];
            let a = (p >> 24) & 0xFF;

            if a > 0 {
                let r = (p >> 16) & 0xFF;
                let g = (p >> 8) & 0xFF;
                let b = p & 0xFF;
                let color_hex = format!("#{r:02X}{g:02X}{b:02X}");

                match config.shape {
                    SvgShape::Rect => {
                        // Optimization: Group contiguous pixels of the same color into a single rect
                        let mut group_width = 1;
                        while x + group_width < width {
                            let next_idx = (y * width + (x + group_width)) as usize;
                            if pixels[next_idx] == p {
                                group_width += 1;
                            } else {
                                break;
                            }
                        }

                        let _ = writeln!(
                            &mut svg,
                            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" />",
                            x as usize * scale,
                            y_scaled,
                            group_width as usize * scale,
                            scale,
                            color_hex
                        );
                        x += group_width - 1; // Advance x by the group size (minus 1, since the loop will increment it)
                    }
                    SvgShape::Circle => {
                        let r_scale = scale as f32 / 2.0;
                        let cx = (x as usize * scale) as f32 + r_scale;
                        let cy = y_scaled as f32 + r_scale;

                        let _ = writeln!(
                            &mut svg,
                            "  <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r_scale}\" fill=\"{color_hex}\" />"
                        );
                    }
                }
            }
            x += 1;
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_to_svg_basic() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF0000); // Red
        fb.set_pixel(1, 0, 0xFF0000FF); // Blue

        let config = SvgExportConfig::default();
        let svg_str = export_to_svg(&fb, &config);

        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("<rect"));
        assert!(svg_str.contains("#FF0000"));
        assert!(svg_str.contains("#0000FF"));
    }
    #[test]
    fn test_export_to_svg_circle_and_scale() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF0000); // Red

        let config = SvgExportConfig {
            shape: SvgShape::Circle,
            scale: 10,
        };
        let svg_str = export_to_svg(&fb, &config);

        assert!(svg_str.contains("<svg width=\"10\" height=\"10\""));
        assert!(svg_str.contains("<circle cx=\"5\" cy=\"5\" r=\"5\" fill=\"#FF0000\" />"));
    }
}
