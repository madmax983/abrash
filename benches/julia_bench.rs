use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::mandelbrot::{JuliaConfig, render_julia};

#[cfg(feature = "nova")]
fn bench_julia(c: &mut Criterion) {
    let mut group = c.benchmark_group("Julia Renderer");
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = JuliaConfig::default();

    group.bench_function("render_julia", |b| {
        b.iter(|| {
            render_julia(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_julia(_c: &mut Criterion) {
    // Benchmark does nothing when the feature is disabled
}

criterion_group!(benches, bench_julia);
criterion_main!(benches);
