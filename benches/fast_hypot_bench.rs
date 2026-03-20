use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_hypot(c: &mut Criterion) {
    let mut group = c.benchmark_group("hypot");

    let mut inputs = Vec::with_capacity(1000);
    for y in -15..15 {
        for x in -15..15 {
            inputs.push((y as f32 * 1.5, x as f32 * 1.5));
        }
    }

    group.bench_function("std_hypot", |b| {
        b.iter(|| {
            for (y, x) in &inputs {
                black_box(black_box(*x).hypot(black_box(*y)));
            }
        });
    });

    group.bench_function("fast_hypot", |b| {
        b.iter(|| {
            for (y, x) in &inputs {
                #[allow(clippy::imprecise_flops)]
                let res = (black_box(*x) * black_box(*x) + black_box(*y) * black_box(*y)).sqrt();
                black_box(res);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hypot);
criterion_main!(benches);
