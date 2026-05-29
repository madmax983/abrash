#![allow(unused)]
#[cfg(feature = "nova")]
use abrash::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn edge_glow_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with some pattern to avoid optimizing out
    let pixels = fb.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let val = (x + y) % 256;
            pixels[(y * width + x) as usize] = 0xFF00_0000 | (val << 16) | (val << 8) | val;
        }
    }

    let config = EdgeGlowConfig {
        edge_color: 0x00_FF_00_FF, // Magenta
        intensity: 2.0,
        edge_threshold: 50,
        darken_factor: 0.2,
    };

    let mut group = c.benchmark_group("edge_glow");
    group.sample_size(10); // Reduce sample size for faster benchmarking of large resolutions

    group.bench_function("1080p", |b| {
        b.iter(|| {
            apply_edge_glow(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, edge_glow_benchmark);

#[cfg(not(feature = "nova"))]
#[allow(clippy::needless_pass_by_ref_mut, clippy::missing_const_for_fn)]
fn edge_glow_benchmark(c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, edge_glow_benchmark);
criterion_main!(benches);
