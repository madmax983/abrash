#[cfg(feature = "nova")]
use abrash::experimental::radial_blur::apply_radial_blur;
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use criterion::black_box;
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn bench_radial_blur(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 {
                0x00FF_FFFF
            } else {
                0x0000_0000
            };
            fb.set_pixel(x, y, color);
        }
    }

    let mut group = c.benchmark_group("radial_blur");
    group.sample_size(100);

    // Bench SWAR optimized path
    group.bench_function("swar_16_samples", |b| {
        b.iter(|| {
            apply_radial_blur(
                black_box(&mut fb),
                black_box(512),
                black_box(512),
                black_box(0.5),
                black_box(16),
            );
        });
    });

    // Bench scalar fallback path
    group.bench_function("scalar_fallback_257_samples", |b| {
        b.iter(|| {
            apply_radial_blur(
                black_box(&mut fb),
                black_box(512),
                black_box(512),
                black_box(0.5),
                black_box(257),
            );
        });
    });

    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, bench_radial_blur);

#[cfg(not(feature = "nova"))]
#[allow(clippy::missing_const_for_fn, clippy::needless_pass_by_ref_mut)]
fn bench_radial_blur(_c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, bench_radial_blur);
criterion_main!(benches);
