use abrash::experimental::mandelbrot::{MandelbrotConfig, generate_mandelbrot};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = MandelbrotConfig::default();

    c.bench_function("mandelbrot_generate_800x600", |b| {
        b.iter(|| generate_mandelbrot(black_box(&mut fb), black_box(&config)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
