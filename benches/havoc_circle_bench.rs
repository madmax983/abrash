use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::{draw_circle, fill_circle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_circle(c: &mut Criterion) {
    let mut group = c.benchmark_group("circle_havoc");
    let mut fb = Framebuffer::new(100, 100).unwrap();

    group.bench_function("fill_circle_100", |b| {
        b.iter(|| {
            fill_circle(
                black_box(&mut fb),
                black_box(50),
                black_box(50),
                black_box(100),
                black_box(0xFFFFFFFF),
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_circle);
criterion_main!(benches);
