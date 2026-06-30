use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::strange_attractor::render_clifford_attractor;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_attractor(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_00_00_00);

    c.bench_function("clifford_attractor 10M", |b| {
        b.iter(|| {
            render_clifford_attractor(&mut fb, 1.5, -1.8, 1.6, 2.0, 10_000_000);
        });
    });
}

criterion_group!(benches, bench_attractor);
criterion_main!(benches);
