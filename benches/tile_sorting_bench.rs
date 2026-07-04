use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn partial_cmp_sort(data: &mut [f32]) {
    data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
}

fn int_sort(data: &mut [f32]) {
    data.sort_unstable_by_key(|&d| {
        let bits = d.to_bits() as i32;
        bits ^ (bits >> 31 & 0x7FFF_FFFF)
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sorting floats");
    let mut rng = abrash_core::random::Rng::seeded(42);
    let mut data: Vec<f32> = (0..1000).map(|_| rng.f32_range(0.0, 100.0)).collect();

    group.bench_function("partial_cmp", |b| {
        b.iter(|| {
            let mut clone = data.clone();
            partial_cmp_sort(black_box(&mut clone));
            black_box(clone);
        })
    });

    group.bench_function("int_cmp", |b| {
        b.iter(|| {
            let mut clone = data.clone();
            int_sort(black_box(&mut clone));
            black_box(clone);
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
