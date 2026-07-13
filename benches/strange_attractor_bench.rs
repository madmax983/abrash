use abrash::experimental::strange_attractor::render_clifford_attractor;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_strange_attractor(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut group = c.benchmark_group("strange_attractor");
    group.bench_function("render_clifford_1M_iters", |b| {
        b.iter(|| {
            render_clifford_attractor(
                black_box(&mut fb),
                black_box(-1.4),
                black_box(1.6),
                black_box(1.0),
                black_box(0.7),
                black_box(1_000_000),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, bench_strange_attractor);
criterion_main!(benches);
