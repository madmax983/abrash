use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash::experimental::fractal::render_mandelbrot;
use abrash::framebuffer::Framebuffer;

fn bench_mandelbrot(c: &mut Criterion) {
    let mut fb = Framebuffer::new(320, 240).unwrap();

    let mut group = c.benchmark_group("fractal");
    group.bench_function("render_mandelbrot_320x240", |b| {
        b.iter(|| {
            render_mandelbrot(
                black_box(&mut fb),
                black_box(-0.5),
                black_box(0.0),
                black_box(1.5),
                black_box(50),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, bench_mandelbrot);
criterion_main!(benches);
