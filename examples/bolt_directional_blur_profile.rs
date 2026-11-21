//! Deterministic profiling harness for `examples/directional_blur_demo.rs`'s
//! per-frame blur: `apply_directional_blur` on an 800x600 gradient/checker
//! framebuffer with the demo's 16 samples and a motion vector that varies per
//! frame, for N frames (default 3).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. Windowing/present is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::directional_blur::{DirectionalBlurConfig, apply_directional_blur};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let checker = ((x / 40 + y / 40) % 2) * 0x40;
            let c = 0xFF00_0000
                | (((x * 255 / WIDTH) + checker).min(255) << 16)
                | (((y * 255 / HEIGHT) + checker).min(255) << 8)
                | ((x ^ y) & 0xFF);
            original.set_pixel(x as i32, y as i32, c);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for f in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let t = f as f32;
        let config = DirectionalBlurConfig {
            dx: 40.0 - 25.0 * t,
            dy: -30.0 + 20.0 * t,
            num_samples: 16,
        };
        apply_directional_blur(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
