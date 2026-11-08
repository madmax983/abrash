//! Deterministic profiling harness for `examples/speed_lines_demo.rs`'s
//! `apply_speed_lines` call: 800x600 framebuffer, same config as the demo
//! (density 180, inner radius 120, lengths 50..400), time advancing 0.05 per
//! frame, 30 frames. Run under `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::speed_lines::{SpeedLinesConfig, apply_speed_lines};
use std::hint::black_box;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut checksum: u64 = 0;
    for frame in 0..frames {
        for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
            *p = 0xFF00_0000 | (i as u32).wrapping_mul(2_654_435_761) >> 8;
        }
        let config = SpeedLinesConfig {
            center_x: 0.5,
            center_y: 0.5,
            inner_radius: 120.0,
            time: frame as f32 * 0.05,
            line_color: 0xFFFF_FFFF,
            density: 180,
            min_length: 50.0,
            max_length: 400.0,
        };
        apply_speed_lines(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
