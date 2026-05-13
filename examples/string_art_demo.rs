//! String Art Demo
//!
//! Generates a procedural image and applies the String Art (Thread Art) post-processing filter.

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::{apply_string_art, StringArtConfig};
use std::path::Path;

fn main() {
    let width = 512;
    let height = 512;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Generate a test image: a white background with a dark circle
    fb.clear(0xFF_FF_FF_FF); // White background

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let r = 150;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy < r * r {
                fb.set_pixel(x, y, 0xFF_00_00_00); // Black circle
            }
        }
    }

    // Apply the String Art filter
    // Default mode expects a dark target on white background?
    // Wait, the default config expects to draw strings on dark areas to recreate darkness.
    let mut config = StringArtConfig::default();
    config.num_pins = 200;
    config.num_lines = 2500;
    // Let's use red string just for fun!
    config.line_color = 0x22_FF_00_00;

    println!("Applying string art filter...");
    apply_string_art(&mut fb, config);

    // Save the output
    let output_path = "string_art_output.ppm";
    fb.export_ppm(Path::new(output_path)).unwrap();
    println!("Saved output to {}", output_path);
}
