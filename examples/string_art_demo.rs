use abrash::experimental::string_art::{StringArtConfig, apply_string_art};
use abrash::framebuffer::Framebuffer;

fn main() {
    println!("Generating String Art Demo...");

    let width = 512;
    let height = 512;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill the framebuffer with a simple procedural pattern (a dark ring)
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let max_radius = 200.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);

            // Create a dark ring
            let color = if dist > 100.0 && dist < max_radius {
                // Gradient based on distance
                let intensity = (dist - 100.0) / 100.0;
                let v = (255.0 * intensity) as u32;
                0xFF00_0000 | (v << 16) | (v << 8) | v
            } else {
                0xFF_FFFFFF // White background
            };

            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config = StringArtConfig {
        num_pegs: 256,
        num_strings: 2000,
        string_alpha: 0.1,
    };

    println!("Applying String Art filter...");
    apply_string_art(&mut fb, &config);

    println!("String Art generated successfully! (Framebuffer contains the procedural output).");
    println!("In a real engine run, this would be displayed on screen.");
}
