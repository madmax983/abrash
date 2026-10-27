//! Deterministic profiling harness for `apply_crosshatch`: an 800x600
//! framebuffer holding a smooth radial-gradient "photo" (full luminance range,
//! so all four hatch tiers are exercised), hatch spacing 6, N frames (default
//! 30). Each frame re-copies the source since the filter is destructive.
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::experimental::crosshatch::apply_crosshatch;
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
            let dx = x as f32 - 400.0;
            let dy = y as f32 - 300.0;
            let v = ((dx * dx + dy * dy).sqrt() / 500.0).min(1.0);
            let c = (v * 255.0) as u32;
            original.set_pixel(x as i32, y as i32, 0xFF00_0000 | (c << 16) | (c << 8) | c);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_crosshatch(&mut fb, 6);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
