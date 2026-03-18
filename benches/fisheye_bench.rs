//! Benchmark for Fisheye Lens post-processing effect

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::fisheye::apply_fisheye;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fisheye_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("fisheye");

    // Standard resolutions
    let resolutions = [(800, 600, "SVGA"), (1920, 1080, "FHD")];

    for &(width, height, name) in &resolutions {
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Fill with a simple pattern
        for y in 0..height {
            for x in 0..width {
                let color = if (x / 20) % 2 == (y / 20) % 2 {
                    0xFF_FF_FF_FF
                } else {
                    0xFF_00_00_00
                };
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        group.bench_function(format!("{}_{}", "fisheye", name), |b| {
            b.iter(|| {
                // Apply a moderately strong fisheye effect
                apply_fisheye(black_box(&mut fb), black_box(0.5));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, fisheye_benchmark);
criterion_main!(benches);
