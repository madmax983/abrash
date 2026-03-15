//! Demonstration of the Swirl Filter.
//!
//! Creates a colorful grid pattern and applies a continuously twisting Swirl filter.
//!
//! Run with:
//! ```sh
//! cargo run --example swirl_demo --no-default-features --features "backend-tui parallel nova" --release
//! ```

use abrash::experimental::swirl::{apply_swirl, SwirlConfig};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};
use std::f32::consts::PI;

fn main() {
    let mut window = Window::new("Swirl Demo", 800, 600).unwrap();

    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut angle: f32 = 0.0;

    // Draw a checkerboard pattern onto a persistent buffer
    let mut original_fb = Framebuffer::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let cx = x / 50;
            let cy = y / 50;
            let is_white = (cx + cy) % 2 == 0;
            let color = if is_white { 0xFF_FF_FF_FF } else { 0xFF_00_00_AA };
            original_fb.set_pixel(x as i32, y as i32, color);
            // Draw a red center line to make the swirl obvious
            if (x == 400 && y > 100 && y < 500) || (y == 300 && x > 100 && x < 700) {
                original_fb.set_pixel(x as i32, y as i32, 0xFF_FF_00_00);
            }
        }
    }

    while window.is_open() {
        let events = window.poll_events();
        for event in events {
            if matches!(event, Event::Close) {
                return;
            }
        }

        angle += 0.05;
        let twist = angle.sin() * PI; // oscillate between -PI and +PI

        // Reset framebuffer manually using slice copy
        fb.as_mut_slice().copy_from_slice(original_fb.as_slice());

        let config = SwirlConfig {
            center_x: 0.5,
            center_y: 0.5,
            radius: 250.0,
            angle: twist,
        };

        apply_swirl(&mut fb, &config);

        window.blit_framebuffer(&fb);
    }
}
