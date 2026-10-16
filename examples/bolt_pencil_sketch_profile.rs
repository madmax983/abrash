//! Deterministic profiling harness for `examples/pencil_sketch_demo.rs`'s
//! per-frame work: reset an 800x600 framebuffer to a synthetic scene
//! (smooth gradient, discs, and light per-pixel noise), then run
//! `apply_pencil_sketch` with the default config, for N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::pencil_sketch::{PencilSketchConfig, apply_pencil_sketch};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut seed = 0x1234_5678u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (seed >> 27) as u32; // 0..31
            let mut r = x * 255 / WIDTH;
            let mut g = y * 255 / HEIGHT;
            let mut b = 128u32;
            for (cx, cy, rad, c) in [
                (250i32, 200i32, 120i32, (230u32, 60u32, 40u32)),
                (550, 380, 150, (30, 90, 200)),
            ] {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                if dx * dx + dy * dy < rad * rad {
                    (r, g, b) = c;
                }
            }
            let r = (r + noise).min(255);
            let g = (g + noise).min(255);
            let b = (b + noise).min(255);
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }

    let config = PencilSketchConfig::default();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_pencil_sketch(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
