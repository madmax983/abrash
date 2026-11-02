//! Deterministic profiling harness for `apply_neon_outline`: an 800x600
//! scene (smooth gradient background with a few filled discs and rectangles,
//! so edges are sparse as in a real frame, not worst-case noise), repeated
//! for N frames (default 10).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::experimental::neon_outline::{NeonOutlineConfig, apply_neon_outline};
use abrash::framebuffer::Framebuffer;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn scene() -> Framebuffer {
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let r = 40 + (x * 100 / WIDTH);
            let g = 30 + (y * 100 / HEIGHT);
            let b = 80 + ((x + y) * 60 / (WIDTH + HEIGHT));
            fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }
    // Filled discs.
    for &(cx, cy, rad, col) in &[
        (200i32, 180i32, 90i32, 0xFFE0_4030u32),
        (560, 220, 120, 0xFF30_C060),
        (400, 450, 70, 0xFFF0_F0A0),
    ] {
        for y in (cy - rad)..=(cy + rad) {
            for x in (cx - rad)..=(cx + rad) {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= rad * rad {
                    fb.set_pixel(x, y, col);
                }
            }
        }
    }
    // Rectangles.
    for &(x0, y0, w, h, col) in &[
        (60i32, 400i32, 160i32, 120i32, 0xFFFF_FFFFu32),
        (620, 400, 140, 150, 0xFF10_1020),
    ] {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                fb.set_pixel(x, y, col);
            }
        }
    }
    fb
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let original = scene();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let config = NeonOutlineConfig::default();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_neon_outline(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
