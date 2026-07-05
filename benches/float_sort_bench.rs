use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::{Rng, thread_rng};
use std::cmp::Ordering;

fn generate_random_floats(count: usize) -> Vec<f32> {
    let mut rng = thread_rng();
    (0..count).map(|_| rng.gen_range(0.0..100.0)).collect()
}

fn bench_float_sorting(c: &mut Criterion) {
    let count = 10000;

    let mut group = c.benchmark_group("float_sorting");

    group.bench_function("partial_cmp_unwrap_or", |b| {
        b.iter_batched(
            || generate_random_floats(count),
            |mut data| {
                data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("total_cmp", |b| {
        b.iter_batched(
            || generate_random_floats(count),
            |mut data| {
                data.sort_unstable_by(f32::total_cmp);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_float_sorting);
criterion_main!(benches);
