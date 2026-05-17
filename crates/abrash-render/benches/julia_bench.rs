use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::julia::{JuliaConfig, render_julia};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_julia(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = JuliaConfig::default();

    c.bench_function("render_julia 1080p", |b| {
        b.iter(|| {
            render_julia(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, benchmark_julia);
criterion_main!(benches);
