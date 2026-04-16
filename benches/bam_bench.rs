use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::f32::consts::PI;

fn std_sin_cos(c: &mut Criterion) {
    let mut group = c.benchmark_group("SinCos");
    group.bench_function("std_sin_cos", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let a = (i as f32) * PI / 500.0;
                black_box(a.sin_cos());
            }
        })
    });
    group.finish();
}

criterion_group!(benches, std_sin_cos);
criterion_main!(benches);
