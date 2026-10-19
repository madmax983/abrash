//! Deterministic profiling harness for `apply_water_ripple`: 800x600
//! checkerboard + cross pattern, default config with an animated phase and a
//! radius covering most of the frame, for N frames (default 30).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::water_ripple::{RippleConfig, apply_water_ripple};
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
            let is_white = (x / 50 + y / 50) % 2 == 0;
            let mut color = if is_white {
                0xFF_FF_FF_FF
            } else {
                0xFF_00_00_AA
            };
            if (x == 400 && y > 100 && y < 500) || (y == 300 && x > 100 && x < 700) {
                color = 0xFF_FF_00_00;
            }
            original.set_pixel(x as i32, y as i32, color);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut phase = 0.0f32;
    let mut checksum: u64 = 0;

    for _ in 0..frames {
        phase += 0.2;
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let config = RippleConfig {
            phase,
            radius: 0.6,
            ..RippleConfig::default()
        };
        apply_water_ripple(&mut fb, config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
