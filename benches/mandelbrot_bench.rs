use abrash::experimental::mandelbrot::{MandelbrotConfig, apply_mandelbrot};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_mandelbrot(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = MandelbrotConfig::default();

    c.bench_function("mandelbrot_800x600", |b| {
        b.iter(|| {
            apply_mandelbrot(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_mandelbrot);
criterion_main!(benches);
