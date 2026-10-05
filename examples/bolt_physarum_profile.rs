//! Deterministic-counter profiling harness for the Physarum demo's per-frame
//! work (`examples/physarum_demo.rs`): 640x480 framebuffer cleared then
//! `apply_physarum` with the demo's default config (50,000 agents), N frames
//! (default 20, so trails reach a realistic steady-state density).
//!
//! Not a criterion benchmark: meant to be run under
//! `valgrind --tool=callgrind` / `--tool=dhat` for deterministic counters.
//! Note: `apply_physarum` seeds its steering RNG from the system clock, so
//! counts vary slightly (branch mix) between runs; compare several runs.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::physarum::{PhysarumConfig, apply_physarum};
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let config = PhysarumConfig::default();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    for _ in 0..frames {
        fb.clear(0xFF_000000);
        apply_physarum(&mut fb, &config);
    }
    let checksum: u64 = fb.as_slice().iter().map(|&p| u64::from(p)).sum();
    println!("checksum {}", black_box(checksum));
}
