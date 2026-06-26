use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sepia::apply_sepia;

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    c.bench_function("Sepia Filter/1920x1080", |b| b.iter(|| apply_sepia(&mut fb)));
}
criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
