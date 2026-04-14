use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::mandelbrot::{MandelbrotConfig, render_mandelbrot};

#[cfg(feature = "nova")]
fn bench_mandelbrot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mandelbrot Renderer");
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = MandelbrotConfig::default();

    group.bench_function("render_mandelbrot", |b| {
        b.iter(|| {
            render_mandelbrot(black_box(&mut fb), black_box(&config));
        })
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_mandelbrot(_c: &mut Criterion) {
    // Benchmark does nothing when the feature is disabled
}

criterion_group!(benches, bench_mandelbrot);
criterion_main!(benches);
