use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::{StringArtConfig, apply_string_art};

fn main() {
    let width = 500;
    let height = 500;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Create a simple gradient image as source
    for y in 0..height {
        for x in 0..width {
            let cx = x as f32 - width as f32 / 2.0;
            let cy = y as f32 - height as f32 / 2.0;
            let d = cx.hypot(cy);
            let v = (255.0 - d.min(255.0)) as u32;
            fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | (v << 16) | (v << 8) | v);
        }
    }

    let config = StringArtConfig {
        num_pins: 256,
        num_lines: 3000,
        line_opacity: 0.1,
        ..Default::default()
    };

    apply_string_art(&mut fb, &config);
    fb.export_ppm("string_art_demo.ppm").unwrap();
    println!("Exported string_art_demo.ppm");
}
