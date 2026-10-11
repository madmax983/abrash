//! Deterministic profiling harness for `examples/plasma_demo.rs`'s per-frame
//! work: `apply_plasma` on the demo's 640x480 framebuffer with the demo's
//! scale (0.05) and time step (~60 Hz, x2 speed), for N frames (default 60).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::plasma::apply_plasma;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut time = 0.0f32;
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        time += (1.0 / 60.0) * 2.0;
        apply_plasma(&mut fb, std::hint::black_box(time), 0.05);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    println!("frames={frames} checksum={checksum:#018x}");
}
