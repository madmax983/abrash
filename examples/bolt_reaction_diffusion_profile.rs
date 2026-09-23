//! Deterministic profiling harness for `ReactionDiffusion::step`/`render` (Gray-Scott
//! reaction-diffusion simulation).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This binary is
//! meant to be run under `valgrind --tool=callgrind` / `--tool=dhat` to get deterministic
//! instruction and allocation counts for a realistic workload that mirrors
//! `examples/reaction_diffusion_demo.rs`'s per-frame work exactly: a 400x300 grid (the
//! demo runs the sim at half of its 800x600 window resolution), seeded the same way (three
//! fixed seed squares plus one region of `XorShift32` random noise), 10 `step()` calls per
//! frame, followed by one `render()` call, for N frames.
use abrash::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use abrash_render::experimental::reaction_diffusion::ReactionDiffusion;
use std::hint::black_box;

const SIM_WIDTH: usize = 400;
const SIM_HEIGHT: usize = 300;
const COLOR_A: u32 = 0xFF_110022;
const COLOR_B: u32 = 0xFF_00FFAA;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let mut sim = ReactionDiffusion::new(SIM_WIDTH, SIM_HEIGHT);

    let mut rng = XorShift32::new(42);
    sim.seed(SIM_WIDTH / 2, SIM_HEIGHT / 2, 10);
    sim.seed(SIM_WIDTH / 3, SIM_HEIGHT / 3, 5);
    sim.seed(SIM_WIDTH * 2 / 3, SIM_HEIGHT * 2 / 3, 8);
    sim.seed_random(&mut rng, SIM_WIDTH / 4);

    let mut fb = Framebuffer::new(SIM_WIDTH as u32, SIM_HEIGHT as u32).unwrap();

    for _ in 0..frames {
        for _ in 0..10 {
            sim.step();
        }
        sim.render(&mut fb, COLOR_A, COLOR_B);
        black_box(&fb);
    }

    // Checksum to keep the compiler from eliding the work, and to compare before/after
    // pixel output for behavior-preservation.
    let checksum: u64 = fb.as_slice().iter().map(|&p| u64::from(p)).sum();
    println!("frames={frames} checksum={checksum}");
}
