use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_render::experimental::attractor::CliffordAttractor;

pub fn attractor_benchmark(c: &mut Criterion) {
    let a = CliffordAttractor::new(1.5, -1.8, 1.6, 0.9);
    c.bench_function("attractor_single", |b| b.iter(|| {
        let mut x = black_box(0.1);
        let mut y = black_box(0.1);
        for _ in 0..1000 {
            let (nx, ny) = a.step(x, y);
            x = nx;
            y = ny;
        }
        black_box((x, y));
    }));
    c.bench_function("attractor_batch", |b| b.iter(|| {
        let mut pts = Vec::new();
        a.run_steps(black_box(0.1), black_box(0.1), black_box(1000), &mut pts);
    }));
}

criterion_group!(benches, attractor_benchmark);
criterion_main!(benches);
