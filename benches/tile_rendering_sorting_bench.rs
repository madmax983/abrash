use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_float_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("float_sorting");

    // Generate highly clustered, overlapping depth values (simulate a tile with lots of geometry)
    let depths: Vec<f32> = (0..10_000).map(|i| {
        100.0 + ((i % 100) as f32) * 0.001
    }).collect();

    group.bench_function("partial_cmp", |b| {
        b.iter(|| {
            let mut data = depths.clone();
            data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            black_box(data);
        })
    });

    group.bench_function("total_cmp", |b| {
        b.iter(|| {
            let mut data = depths.clone();
            data.sort_unstable_by(|a, b| a.total_cmp(b));
            black_box(data);
        })
    });

    group.bench_function("integer_mapping_key", |b| {
        b.iter(|| {
            let mut data = depths.clone();
            data.sort_unstable_by_key(|&depth| {
                let bits = depth.to_bits() as i32;
                bits ^ (bits >> 31 & 0x7FFF_FFFF)
            });
            black_box(data);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_float_sort);
criterion_main!(benches);
