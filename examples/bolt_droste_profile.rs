//! Deterministic profiling harness for `examples/droste_demo.rs`'s per-frame
//! work: `apply_droste` on an 800x600 gradient/checker scene with the demo's
//! animated config (5 iterations, oscillating scale/offset), applied
//! to a fresh copy of the source for N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::droste::{DrosteConfig, apply_droste};
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
            let checker = if (x / 40 + y / 40) % 2 == 0 { 0x40 } else { 0 };
            let color = 0xFF00_0000
                | (((x * 255 / WIDTH) + checker).min(255) << 16)
                | ((y * 255 / HEIGHT) << 8)
                | 0x80;
            original.set_pixel(x as i32, y as i32, color);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        let elapsed = frame as f32 * (1.0 / 30.0);
        let config = DrosteConfig {
            iterations: 5,
            scale: 0.5 + (elapsed.sin() * 0.2),
            offset_x: (elapsed * 0.5).cos() * 0.2,
            offset_y: (elapsed * 0.5).sin() * 0.2,
        };
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_droste(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
