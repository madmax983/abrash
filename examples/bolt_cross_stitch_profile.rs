//! Deterministic profiling harness for `apply_cross_stitch`: the
//! `cross_stitch_demo` configuration (640x480 sinusoidal background,
//! `cell_size` 10, canvas 0xFFEFEFEF), repeated for N frames (default 10).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`.
use abrash::experimental::cross_stitch::{CrossStitchConfig, apply_cross_stitch};
use abrash::framebuffer::Framebuffer;
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn scene() -> Framebuffer {
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let time = 1.3f32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut u = x as f32 / WIDTH as f32;
            let mut v = y as f32 / HEIGHT as f32;
            u += (time * 0.5).sin() * 0.2;
            v += (time * 0.3).cos() * 0.2;
            let r = ((u * std::f32::consts::PI * 4.0).sin() * 127.0 + 128.0) as u32;
            let g = ((v * std::f32::consts::PI * 4.0).cos() * 127.0 + 128.0) as u32;
            let b = (((u + v) * std::f32::consts::PI * 2.0).sin() * 127.0 + 128.0) as u32;
            fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
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
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        let config = CrossStitchConfig {
            cell_size: 10,
            canvas_color: 0xFF_EFEFEF,
        };
        apply_cross_stitch(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
