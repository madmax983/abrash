use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::{FilmGrainConfig, apply_film_grain};
use std::time::Instant;

fn main() {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF808080);
    let config = FilmGrainConfig {
        intensity: 0.25,
        seed: 0xBADF00D,
    };

    let start = Instant::now();
    for _ in 0..100 {
        apply_film_grain(&mut fb, &config);
    }
    let duration = start.elapsed();
    println!("Time for 100 iterations: {duration:?}");
    println!("Average time per iteration: {:?}", duration / 100);
}
