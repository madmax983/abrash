use abrash::experimental::pencil_sketch::{PencilSketchConfig, apply_pencil_sketch};
use abrash_core::framebuffer::Framebuffer;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a scene (checkerboard and gradient)
    for y in 0..height {
        for x in 0..width {
            let cx = x / 50;
            let cy = y / 50;
            let val = if (cx + cy) % 2 == 0 { 200 } else { 50 };

            // Add gradient from left to right to test hatching
            let grad = (x as f32 / width as f32 * 255.0) as u32;
            let mixed = (val + grad) / 2;

            let color = 0xFF00_0000 | (mixed << 16) | (mixed << 8) | mixed;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Draw some sharp lines to trigger edge detection
    for i in 0..100 {
        fb.set_pixel(400 + i as i32, 300 + i as i32, 0xFF000000);
        fb.set_pixel(400 - i as i32, 300 + i as i32, 0xFF000000);
    }

    let config = PencilSketchConfig::default();

    println!("Applying Pencil Sketch effect...");
    let start = Instant::now();
    apply_pencil_sketch(&mut fb, &config);
    let duration = start.elapsed();

    println!("Effect applied in {:?}", duration);
    println!("Example complete.");
}
