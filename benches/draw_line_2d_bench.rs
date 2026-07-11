use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::draw_line_2d;

pub fn draw_line_2d_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    c.bench_function("draw_line_2d", |b| b.iter(|| {
        draw_line_2d(&mut fb, black_box(10), black_box(10), black_box(1900), black_box(1000), black_box(0xFFFF_FFFF));
    }));
}
criterion_group!(benches, draw_line_2d_benchmark);
criterion_main!(benches);
