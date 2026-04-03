#[cfg(feature = "nova")]
use abrash::experimental::plasma::apply_plasma;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn plasma_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut group = c.benchmark_group("plasma");
    group.sample_size(10); // Reduce sample size for faster benchmarking of large resolutions

    group.bench_function("1080p", |b| {
        b.iter(|| {
            apply_plasma(black_box(&mut fb), black_box(1.0), black_box(0.05));
        });
    });

    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, plasma_benchmark);

#[cfg(not(feature = "nova"))]
fn plasma_benchmark(c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, plasma_benchmark);
criterion_main!(benches);