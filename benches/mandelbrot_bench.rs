use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::mandelbrot::render_mandelbrot;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_render_mandelbrot(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("mandelbrot_800x600_100iter", |b| {
        b.iter(|| {
            render_mandelbrot(&mut fb, -0.5, 0.0, 1.0, 100);
        });
    });
}

criterion_group!(benches, bench_render_mandelbrot);
criterion_main!(benches);
