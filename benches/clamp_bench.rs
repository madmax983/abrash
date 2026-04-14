use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_clamp(c: &mut Criterion) {
    let mut group = c.benchmark_group("clamp_methods");

    group.bench_function("std_clamp", |b| {
        b.iter(|| {
            for v in 0..10_000 {
                let _ = black_box(v).clamp(0, 5000);
            }
        });
    });

    group.bench_function("manual_clamp", |b| {
        b.iter(|| {
            for v in 0..10_000 {
                let _ = black_box(v).max(0).min(5000);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clamp);
criterion_main!(benches);
