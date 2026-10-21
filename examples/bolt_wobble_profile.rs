//! Deterministic profiling harness for `examples/wobble_demo.rs`'s per-frame
//! work: copy a 640x480 checkerboard background into the framebuffer, then
//! `apply_wobble` with the demo's config and advancing time, for N frames
//! (default 60).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::wobble::{WobbleConfig, apply_wobble};
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut background = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let color = if (x / 32 + y / 32) % 2 == 0 {
                0xFF_22_22_22
            } else {
                0xFF_DD_DD_DD
            };
            background.set_pixel(x as i32, y as i32, color);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut config = WobbleConfig {
        amplitude: 20.0,
        frequency: 4.0,
        time: 0.0,
    };
    let mut checksum: u64 = 0;

    for _ in 0..frames {
        config.time += 1.0 / 60.0;
        fb.as_mut_slice().copy_from_slice(background.as_slice());
        apply_wobble(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
