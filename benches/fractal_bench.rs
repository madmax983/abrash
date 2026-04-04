use abrash_render::experimental::fractal::render_mandelbrot;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_fractal(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut buffer = vec![0; width * height];

    c.bench_function("fractal_mandelbrot_640x480", |b| {
        b.iter(|| {
            render_mandelbrot(black_box(&mut buffer), black_box(width), black_box(height));
        });
    });
}

criterion_group!(benches, bench_fractal);
criterion_main!(benches);
