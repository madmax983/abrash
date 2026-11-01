//! Deterministic profiling harness for `examples/vhs_demo.rs`'s `apply_vhs`
//! call: 640x480 framebuffer (the demo's size), same config as the demo
//! (thickness 0.15, noise 0.25, pulsing intensity, rolling tracking band),
//! 30 frames. Not a criterion benchmark: run under
//! `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::vhs::{VhsConfig, apply_vhs};
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
            *p = 0xFF00_0000 | (i as u32).wrapping_mul(2_654_435_761) >> 8;
        }
        let time = frame as f32 / 60.0 * 4.0;
        let raw_pos = (time * 0.2) % 1.2;
        let config = VhsConfig {
            intensity: 0.5 + (time * 2.0).sin() * 0.2,
            time,
            tracking_position: raw_pos - 0.1,
            tracking_thickness: 0.15,
            noise_intensity: 0.25,
        };
        apply_vhs(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
