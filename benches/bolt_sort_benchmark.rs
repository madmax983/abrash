use abrash_core::random::Rng;
use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

fn bench_float_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("Float Sorting");

    // Generate random floats
    let mut rng = Rng::seeded(42);
    let mut floats: Vec<f32> = Vec::with_capacity(10000);
    for _ in 0..10000 {
        floats.push(rng.f32_range(0.0, 1000.0));
    }

    group.bench_function("sort_unstable_by partial_cmp.unwrap_or", |b| {
        b.iter_batched(
            || floats.clone(),
            |mut data| {
                data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                black_box(data);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("sort_unstable_by total_cmp", |b| {
        b.iter_batched(
            || floats.clone(),
            |mut data| {
                data.sort_unstable_by(f32::total_cmp);
                black_box(data);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_float_sorting);
criterion_main!(benches);
