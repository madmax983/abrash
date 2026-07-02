use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};

fn main() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_222222);

    // Draw some simple colors
    for y in 0..600 {
        for x in 0..800 {
            if x < 400 {
                fb.set_pixel(x, y, 0xFF_FF0000);
            } else {
                fb.set_pixel(x, y, 0xFF_0000FF);
            }
        }
    }

    let config = PointillismConfig {
        cell_size: 15,
        dot_scale: 1.2,
        jitter: 0.5,
    };

    apply_pointillism(&mut fb, &config);
    println!("Pointillism demo ran successfully");
}
