use abrash_render::experimental::strange_attractor::generate_lorenz_attractor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_strange_attractor(c: &mut Criterion) {
    c.bench_function("generate_lorenz_attractor_10000_iters", |b| {
        b.iter(|| {
            let mesh = generate_lorenz_attractor(
                black_box(10000),
                black_box(0.01),
                black_box(10.0),
                black_box(28.0),
                black_box(8.0 / 3.0),
            );
            black_box(mesh);
        });
    });
}

criterion_group!(benches, bench_strange_attractor);
criterion_main!(benches);
