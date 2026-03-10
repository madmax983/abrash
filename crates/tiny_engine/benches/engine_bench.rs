use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tiny_engine::Framebuffer;

fn benchmark_clear(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080);
    c.bench_function("framebuffer_clear", |b| {
        b.iter(|| fb.clear(black_box(0xFF00FF00)))
    });
}

criterion_group!(benches, benchmark_clear);
criterion_main!(benches);
