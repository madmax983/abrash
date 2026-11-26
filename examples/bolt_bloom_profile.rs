//! Deterministic profiling harness for `apply_bloom`: an 800x600 framebuffer
//! with a dim gradient background and scattered bright "lights", default
//! `BloomConfig`, N frames (default 3). Each frame re-copies the source since
//! the filter is destructive.
//!
//! Run under `valgrind --tool=callgrind`. Wall-clock is not used.
use abrash::framebuffer::Framebuffer;
use abrash::post_process::bloom::{BloomConfig, apply_bloom};
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
            let r = (x * 255 / WIDTH) / 2;
            let g = (y * 255 / HEIGHT) / 2;
            let b = ((x + y) * 255 / (WIDTH + HEIGHT)) / 2;
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }
    // Scattered bright lights.
    let mut s: u32 = 12345;
    for _ in 0..40 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let cx = (s >> 8) % WIDTH;
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let cy = (s >> 8) % HEIGHT;
        for dy in 0..12u32 {
            for dx in 0..12u32 {
                let (x, y) = (cx + dx, cy + dy);
                if x < WIDTH && y < HEIGHT {
                    original.set_pixel(x as i32, y as i32, 0xFFF0_F0E0);
                }
            }
        }
    }
    let config = BloomConfig::default();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_bloom(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
