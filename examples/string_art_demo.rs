use abrash::framebuffer::Framebuffer;
use abrash::experimental::string_art::{apply_string_art, StringArtConfig};

fn main() {
    let mut fb = Framebuffer::new(512, 512).unwrap();
    // Fill with a test pattern (a black ring)
    let cx = 256.0_f32;
    let cy = 256.0_f32;
    for y in 0..512 {
        for x in 0..512 {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            let c = if dist > 100.0 && dist < 200.0 { 0xFF_00_00_00 } else { 0xFF_FF_FF_FF };
            fb.set_pixel(x, y, c);
        }
    }

    let config = StringArtConfig {
        num_pins: 200,
        num_lines: 3000,
        ..Default::default()
    };

    apply_string_art(&mut fb, &config);
    println!("String Art applied successfully to 512x512 framebuffer.");
}
