//! Deterministic profiling harness for `apply_fisheye`: 800x600 framebuffer
//! holding a grid + diagonal-gradient pattern, strength 0.5, re-filled from a
//! pristine copy each frame, N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::fisheye::apply_fisheye;
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
            let grid = x % 40 == 0 || y % 40 == 0;
            let color = if grid {
                0xFF_FF_FF_FF
            } else {
                0xFF00_0000
                    | (((x * 255 / WIDTH) & 0xFF) << 16)
                    | (((y * 255 / HEIGHT) & 0xFF) << 8)
                    | ((x ^ y) & 0xFF)
            };
            original.set_pixel(x as i32, y as i32, color);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_fisheye(&mut fb, 0.5);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
