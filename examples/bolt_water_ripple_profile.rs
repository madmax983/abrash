//! Deterministic profiling harness for `apply_water_ripple`: an 800x600
//! gradient framebuffer (as in `benches/water_ripple_bench.rs`), with the
//! bench's config and an advancing phase, for N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::experimental::water_ripple::{RippleConfig, apply_water_ripple};
use abrash::framebuffer::Framebuffer;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (x << 16) | (y << 8));
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for i in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let config = RippleConfig {
            center_x: 0.5,
            center_y: 0.5,
            amplitude: 15.0,
            frequency: 30.0,
            phase: 1.0 + i as f32 * 0.1,
            radius: 0.6,
        };
        apply_water_ripple(&mut fb, config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
