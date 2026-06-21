use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::histogram::apply_histogram_equalization;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with low contrast noise
    for y in 0..height {
        for x in 0..width {
            let r = (x % 50) + 100;
            let g = (y % 50) + 100;
            let b = ((x + y) % 50) + 100;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    println!("Applying histogram equalization to an 800x600 image...");
    let start = Instant::now();
    apply_histogram_equalization(&mut fb);
    let duration = start.elapsed();
    println!("Applied in {duration:?}");

    let sample = fb.get_pixel(400, 300).unwrap();
    println!("Sample pixel after equalization: {sample:#010X}");
}
