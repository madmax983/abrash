use abrash_render::experimental::plasma::apply_plasma;
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    c.bench_function("plasma 800x600", |b| {
        b.iter(|| apply_plasma(black_box(&mut fb), black_box(0.5), black_box(0.05)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
