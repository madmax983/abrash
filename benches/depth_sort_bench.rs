use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{thread_rng, Rng};

fn generate_random_f32_depths(count: usize) -> Vec<f32> {
    let mut rng = thread_rng();
    (0..count).map(|_| rng.gen_range(1.0..1000.0)).collect()
}

fn bench_depth_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("depth_sorting_methods");

    // Using a typical number of triangles that might be sorted in a single tile (e.g., 64)
    let depths = generate_random_f32_depths(64);

    group.bench_function("partial_cmp_unwrap", |b| {
        b.iter(|| {
            let mut data = depths.clone();
            data.sort_unstable_by(|&a, &b| {
                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
            });
            black_box(data);
        });
    });

    group.bench_function("integer_bitwise_key", |b| {
        b.iter(|| {
            let mut data = depths.clone();
            data.sort_unstable_by_key(|&depth| {
                let bits = depth.to_bits() as i32;
                bits ^ (bits >> 31 & 0x7FFF_FFFF)
            });
            black_box(data);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_depth_sorting);
criterion_main!(benches);
