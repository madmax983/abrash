//! Deterministic profiling harness for `examples/mandelbrot_demo.rs`'s
//! per-frame work: `render_mandelbrot` on the demo's 800x600 framebuffer with
//! the demo's default config and its auto-zoom (`zoom *= 1.01` per frame),
//! for N frames (default 60).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped;
//! everything else mirrors the demo exactly.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::mandelbrot::{MandelbrotConfig, render_mandelbrot};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut config = MandelbrotConfig::default();

    let mut checksum: u64 = 0;
    for _ in 0..frames {
        config.zoom *= 1.01;
        render_mandelbrot(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
