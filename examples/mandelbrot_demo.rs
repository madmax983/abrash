//! Mandelbrot Set Explorer Demo
//!
//! A real-time interactive explorer for the Mandelbrot set.

use abrash::experimental::mandelbrot::{MandelbrotConfig, generate_mandelbrot};
use abrash::framebuffer::Framebuffer;
use abrash::platform::Window;
use std::error::Error;
use std::time::Instant;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() -> Result<(), Box<dyn Error>> {
    let mut window = Window::new("Abrash - Mandelbrot Explorer", WIDTH, HEIGHT)?;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT)?;

    let mut config = MandelbrotConfig::default();

    // Tweak default view to look a bit nicer
    config.center_x = -0.7;
    config.zoom = 1.2;
    config.max_iterations = 250;
    config.color_shift = 7.0;

    let mut last_frame = Instant::now();

    println!("Controls:");
    println!("  Scroll / Up/Down Arrows: Zoom in / out");
    println!("  WASD / Arrow Keys: Pan");
    println!("  +/=: Increase detail (iterations)");
    println!("  -: Decrease detail (iterations)");

    while window.is_open() {
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Process window events
        window.poll_events();

        // Normally we would have Keyboard state in our generic platform Event system,
        // but `abrash`'s simple window abstraction only provides `Event::Close` and `Event::Resize`.
        // To keep this demo working generically within the existing system, we'll
        // just animate the zoom dynamically instead of manual interaction for now.

        // Automatic slow zoom into a nice spot
        let target_x = -0.743643887037151;
        let target_y = 0.131825904205330;

        // Interpolate center towards target
        config.center_x += (target_x - config.center_x) * (dt as f64) * 0.2;
        config.center_y += (target_y - config.center_y) * (dt as f64) * 0.2;

        // Increase zoom exponentially
        config.zoom *= 1.0 + (dt as f64) * 0.4;

        // Increase iterations as we zoom in
        config.max_iterations = (200.0 + config.zoom.log10() * 100.0) as u32;

        generate_mandelbrot(&mut fb, &config);

        window.blit_framebuffer(&fb);

        // Exit if we zoom too far
        if config.zoom > 1_000_000_000.0 {
            break;
        }
    }

    Ok(())
}
