use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};
use std::time::Instant;

fn main() {
    println!("Pointillism Filter Demo");
    let mut src_fb = Framebuffer::new(400, 300).unwrap();
    let mut fb = Framebuffer::new(400, 300).unwrap();

    // Fill source with some pattern
    for y in 0..300 {
        for x in 0..400 {
            let r = (x as f32 / 400.0 * 255.0) as u32;
            let g = (y as f32 / 300.0 * 255.0) as u32;
            let b = 128;
            src_fb.set_pixel(x, y, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }

    let config = PointillismConfig::default();

    let start = Instant::now();
    apply_pointillism(&mut fb, &src_fb, &config);
    let duration = start.elapsed();

    println!("Applied pointillism to 400x300 image in {duration:?}");
}
