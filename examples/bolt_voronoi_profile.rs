//! Deterministic profiling harness for `examples/voronoi_demo.rs`'s
//! `apply_voronoi` call: 200 seeds, image-colored cells, a non-integer
//! metric in [1, 2] (the demo sweeps `1.0 + phase`) and a border thickness
//! in [0, 2], on a smaller framebuffer than the demo's 800x600 (per-pixel
//! cost is size-independent; this keeps callgrind runtime sane).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. Windowing/present and the cube
//! rasterization are skipped; the source image is a deterministic pattern.
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::voronoi::{VoronoiConfig, apply_voronoi};
use std::hint::black_box;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 150;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
            let i = i as u32;
            *p = 0xFF00_0000 | (i.wrapping_mul(2_654_435_761) >> 8);
        }
        // Mirrors the demo's phase sweep: phase in (0, 1).
        let phase = (frame as f32 + 0.5) / frames as f32;
        let config = VoronoiConfig {
            num_seeds: 200,
            use_image_color: true,
            metric: 1.0 + phase,
            seed: frame as u32 * 10,
            border_thickness: phase * 2.0,
            border_color: 0xFF00_0000,
        };
        apply_voronoi(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
