//! SVG Halftone Exporter
//!
//! Converts a framebuffer into a scalable vector graphics (SVG) halftone representation.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub struct SvgHalftoneConfig {
    pub dot_spacing: usize,
    pub max_dot_radius: f32,
    pub background_color: u32,
    pub dot_color: u32,
    pub invert: bool,
}

impl Default for SvgHalftoneConfig {
    fn default() -> Self {
        Self {
            dot_spacing: 10,
            max_dot_radius: 4.5,
            background_color: 0xFF_FFFFFF,
            dot_color: 0xFF_000000,
            invert: false,
        }
    }
}

fn to_hex_color(argb: u32) -> String {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    format!("#{r:02X}{g:02X}{b:02X}")
}

#[must_use]
pub fn export_framebuffer_to_svg_halftone(fb: &Framebuffer, config: &SvgHalftoneConfig) -> String {
    let width = fb.width();
    let height = fb.height();

    let mut svg = String::with_capacity((width * height) as usize * 10);
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );

    // Background
    let bg_color = to_hex_color(config.background_color);
    let _ = write!(
        svg,
        "  <rect width=\"100%\" height=\"100%\" fill=\"{bg_color}\" />\n"
    );

    let dot_spacing = config.dot_spacing;
    let dot_color_hex = to_hex_color(config.dot_color);

    for y in (0..height).step_by(dot_spacing) {
        for x in (0..width).step_by(dot_spacing) {
            // Find average luminance of this cell
            let mut sum_lum = 0u32;
            let mut count = 0;

            for dy in 0..dot_spacing {
                for dx in 0..dot_spacing {
                    let cx = x as usize + dx;
                    let cy = y as usize + dy;

                    if cx < width as usize && cy < height as usize {
                        if let Some(color) = fb.get_pixel(cx as i32, cy as i32) {
                            sum_lum += u32::from(pixel_luminance(color));
                            count += 1;
                        }
                    }
                }
            }

            if count > 0 {
                let avg_lum = sum_lum / count as u32;

                // Halftone standard: high luminance (bright) = small dot
                // invert=true flips this
                let lum_normalized = avg_lum as f32 / 255.0;
                let intensity = if config.invert {
                    lum_normalized
                } else {
                    1.0 - lum_normalized
                };

                let radius = config.max_dot_radius * intensity;

                if radius > 0.1 {
                    let cx = x + dot_spacing as u32 / 2;
                    let cy = y + dot_spacing as u32 / 2;
                    let _ = write!(
                        svg,
                        "  <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius:.2}\" fill=\"{dot_color_hex}\" />\n"
                    );
                }
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_halftone_export() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF_FFFFFF); // White background

        let config = SvgHalftoneConfig {
            dot_spacing: 10,
            max_dot_radius: 5.0,
            background_color: 0xFF_FFFFFF,
            dot_color: 0xFF_000000,
            invert: false,
        };

        let svg_output = export_framebuffer_to_svg_halftone(&fb, &config);

        assert!(svg_output.starts_with("<svg"));
        assert!(svg_output.contains("width=\"20\" height=\"20\""));
        // White background with invert=false -> high luminance -> intensity = 0.0 -> no circles > 0.1
        assert!(!svg_output.contains("<circle"));
    }

    #[test]
    fn test_svg_halftone_black() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF_000000); // Black background

        let config = SvgHalftoneConfig {
            dot_spacing: 10,
            max_dot_radius: 5.0,
            background_color: 0xFF_FFFFFF,
            dot_color: 0xFF_000000,
            invert: false,
        };

        let svg_output = export_framebuffer_to_svg_halftone(&fb, &config);

        // Black background with invert=false -> low luminance -> intensity = 1.0 -> large circles
        assert!(svg_output.contains("<circle"));
        assert!(svg_output.contains("r=\"5.00\""));
    }
}
