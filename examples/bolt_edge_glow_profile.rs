//! Deterministic profiling harness for `examples/edge_glow_demo.rs`'s work:
//! build the demo's 800x600 scene (dark background, white square, dithered
//! stripe block) and run `apply_edge_glow` with the demo's config, N frames.
//!
//! Run under `valgrind --tool=callgrind`. Windowing/present is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let config = EdgeGlowConfig {
        edge_color: 0x00FF_00FF,
        intensity: 2.0,
        edge_threshold: 30,
        darken_factor: 0.1,
    };
    let mut original = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    original.clear(0xFF20_2020);
    for y in 100..200 {
        for x in 100..200 {
            original.set_pixel(x, y, 0xFFFF_FFFF);
        }
    }
    for y in 300..400 {
        for x in 300..500 {
            if (x + y) % 10 < 5 {
                original.set_pixel(x, y, 0xFFAA_AAAA);
            }
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        fb.as_mut_slice().copy_from_slice(original.as_slice());
        apply_edge_glow(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
