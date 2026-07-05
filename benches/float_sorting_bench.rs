use criterion::{Criterion, criterion_group, criterion_main};
use rand::{Rng, thread_rng};
use std::cmp::Ordering;

fn bench_float_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("float_sorting");

    let count = 10_000;
    let mut rng = thread_rng();
    let original_data: Vec<f32> = (0..count).map(|_| rng.gen_range(0.0..100.0)).collect();

    group.bench_function("partial_cmp_unwrap_or", |b| {
        b.iter_batched(
            || original_data.clone(),
            |mut data| {
                data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("total_cmp", |b| {
        b.iter_batched(
            || original_data.clone(),
            |mut data| {
                data.sort_unstable_by(f32::total_cmp);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_float_sorting);
criterion_main!(benches);
