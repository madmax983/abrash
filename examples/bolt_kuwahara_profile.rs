//! Deterministic profiling harness for `apply_kuwahara` (Kuwahara painterly
//! post-processing filter: for each pixel, computes the mean/variance of 4
//! overlapping quadrant regions and keeps the mean of the lowest-variance one).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This
//! binary is meant to be run under `valgrind --tool=callgrind` / `--tool=dhat`
//! to get deterministic instruction and allocation counts for a realistic
//! workload — an 800x600 framebuffer (matching `benches/kuwahara_bench.rs`'s
//! resolution) filled with a procedural pattern that has real local color
//! variance (sinusoidal color bands plus a checker pattern), so the filter
//! does genuine per-pixel min-variance work rather than degenerating to a
//! solid-color fast case. The pattern is perturbed by a per-frame phase so no
//! two frames are identical (avoids constant-folding).
//!
//! `apply_kuwahara` has no dedicated interactive demo; this harness calls the
//! same public API (`abrash::experimental::kuwahara::apply_kuwahara`) that
//! `benches/kuwahara_bench.rs` and `tests/kuwahara_tests.rs` exercise, on a
//! realistic full-framebuffer input, matching how any consumer of this
//! post-processing effect would call it.
use abrash::experimental::kuwahara::apply_kuwahara;
use abrash::framebuffer::Framebuffer;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// Fills `fb` with a procedural pattern (sinusoidal color bands over a coarse
/// checkerboard) that has genuine local variance in every region, so no
/// quadrant is degenerately uniform. `phase` shifts the pattern per frame.
fn fill_pattern(fb: &mut Framebuffer, phase: u32) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let checker = if ((x / 24) + (y / 24) + phase / 7).is_multiple_of(2) {
                40
            } else {
                0
            };
            let r = x.wrapping_add(phase * 3) % 256;
            let g = y.wrapping_add(phase * 5) % 256;
            let b = (x + y).wrapping_add(phase * 2) % 256;
            let r = r.saturating_add(checker).min(255);
            let g = g.saturating_add(checker).min(255);
            let b = b.saturating_add(checker).min(255);
            unsafe {
                fb.set_pixel_unchecked(
                    x as usize,
                    y as usize,
                    0xFF00_0000 | (r << 16) | (g << 8) | b,
                );
            }
        }
    }
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let radius: i32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;

    for frame in 0..frames {
        fill_pattern(&mut fb, frame as u32);
        apply_kuwahara(black_box(&mut fb), black_box(radius));
        checksum = checksum.wrapping_add(fb.as_slice().iter().map(|&p| u64::from(p)).sum::<u64>());
    }

    println!("frames={frames} radius={radius} checksum={checksum}");
}
