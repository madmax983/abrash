//! Deterministic profiling harness for `examples/frosted_glass_demo.rs`'s
//! per-frame work: `apply_frosted_glass` on the demo's 800x600 framebuffer
//! (intensity 10.0, seed `1337 + frame`) for N frames (default 60).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::frosted_glass::apply_frosted_glass;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = 0xFF00_0000
                | ((x * 255 / WIDTH) << 16)
                | ((y * 255 / HEIGHT) << 8)
                | ((x ^ y) & 0xFF);
            original.set_pixel(x as i32, y as i32, c);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_frosted_glass(&mut fb, 10.0, 1337 + frame);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
