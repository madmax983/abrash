//! Deterministic profiling harness for `apply_hologram`: an 800x600
//! gradient+noise framebuffer, default `HologramConfig` with an advancing
//! `time`, for N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::hologram::{HologramConfig, apply_hologram};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut seed = 0x1234_5678_u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (seed >> 24) & 0x1F;
            let r = (x * 255 / WIDTH + noise).min(255);
            let g = (y * 255 / HEIGHT + noise).min(255);
            let b = ((x + y) * 255 / (WIDTH + HEIGHT) + noise).min(255);
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let config = HologramConfig {
            time: frame as f32 * 0.05,
            ..HologramConfig::default()
        };
        apply_hologram(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
