//! Deterministic profiling harness for `apply_tilt_shift`: an 800x600
//! gradient+noise framebuffer, default config, N frames (default 30) with a
//! moving focus plane.
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::experimental::tilt_shift::{TiltShiftConfig, apply_tilt_shift};
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
    let mut s: u32 = 0x1234_5678;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (s >> 24) & 0x3F;
            let r = ((x * 255 / WIDTH) + n).min(255);
            let g = ((y * 255 / HEIGHT) + n).min(255);
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | n);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for i in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let config = TiltShiftConfig {
            focus_dist: 0.4 + 0.2 * (i as f32 / frames as f32),
            ..TiltShiftConfig::default()
        };
        apply_tilt_shift(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
