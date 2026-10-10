//! Deterministic profiling harness for `examples/melt_demo.rs`'s `apply_melt`
//! call: 640x480 framebuffer (the demo's size), melt time swept across the
//! whole transition. Not a criterion benchmark: run under
//! `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::melt::{MeltConfig, apply_melt};
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut config = MeltConfig::new(50.0, 0xFF00_0000);
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
            *p = 0xFF00_0000 | (i as u32).wrapping_mul(2_654_435_761) >> 8;
        }
        config.time = frame as f32 * 0.1;
        apply_melt(&mut fb, &mut config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
