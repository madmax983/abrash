use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::ansi_export::export_to_ansi;

fn main() {
    let width = 64;
    let height = 32;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw a simple gradient background
    for y in 0..height {
        for x in 0..width {
            let r = x * 255 / width;
            let g = y * 255 / height;
            let b = 128;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Draw a "Nova" star in the middle
    let cx = (width / 2) as i32;
    let cy = (height / 2) as i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let dx = x - cx;
            let dy = y - cy;
            // A simple 4-point star distance field approximation
            let dist = (dx.abs() as f32).sqrt() + (dy.abs() as f32).sqrt();
            if dist < 5.0 {
                fb.set_pixel(x, y, 0xFF_FF_FF_00); // Yellow star
            }
        }
    }

    // Export and print
    let ansi_string = export_to_ansi(&fb);
    println!("{ansi_string}");
}
