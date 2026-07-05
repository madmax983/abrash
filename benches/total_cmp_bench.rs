use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;
use std::cmp::Ordering;

fn setup_data(size: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen_range(-1000.0..1000.0)).collect()
}

fn bench_sort_f32(c: &mut Criterion) {
    let size = 10_000;
    let mut group = c.benchmark_group("f32_sorting");

    group.bench_function("partial_cmp_unwrap_or", |b| {
        b.iter_batched(
            || setup_data(size),
            |mut data| {
                data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                black_box(data);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("total_cmp", |b| {
        b.iter_batched(
            || setup_data(size),
            |mut data| {
                data.sort_unstable_by(f32::total_cmp);
                black_box(data);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_sort_f32);
criterion_main!(benches);
