use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::tiny_planet::{TinyPlanetConfig, apply_tiny_planet};
use std::time::Instant;

fn main() {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let config = TinyPlanetConfig::default();

    // Warm-up
    for _ in 0..10 {
        apply_tiny_planet(&mut fb, &config);
    }

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        apply_tiny_planet(&mut fb, &config);
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed / iterations;

    println!("Tiny Planet Filter Benchmark ({}x{})", width, height);
    println!("Total time for {} iterations: {:?}", iterations, elapsed);
    println!("Average time per frame: {:?}", avg_time);
    println!("FPS: {:.2}", 1.0 / avg_time.as_secs_f64());
}
