//! Deterministic profiling harness for `examples/metaballs_demo.rs`'s
//! per-frame work: `Metaballs::update_and_render` on the demo's 800x600
//! framebuffer with the demo's config, for N frames (default 20).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind` / `--tool=dhat`. The windowing/present
//! step is skipped; everything else mirrors the demo exactly.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::metaballs::{Metaballs, MetaballsConfig};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut metaballs = Metaballs::new(MetaballsConfig {
        num_balls: 10,
        threshold: 1.0,
        blob_color: 0xFF_FF00FF,
        bg_color: 0xFF_111111,
        speed: 5.0,
        max_size: 40.0,
    });

    let mut checksum: u64 = 0;
    for _ in 0..frames {
        metaballs.update_and_render(&mut fb);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
