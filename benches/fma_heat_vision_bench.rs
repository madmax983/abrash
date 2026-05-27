use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn apply_heat_vision_baseline(depth: f32, min_z: f32, range: f32) -> u32 {
    let scale = 1024.0 / range;
    let t = ((depth - min_z) * scale) as u32;
    t.min(1023)
}

fn apply_heat_vision_fma(depth: f32, min_z: f32, range: f32) -> u32 {
    let scale = 1024.0 / range;
    let offset = min_z * scale;
    let t = (depth * scale - offset) as u32;
    t.min(1023)
}

fn bench_fma_vs_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision FMA Math");
    let depth = 3.5;
    let min_z = 1.0;
    let range = 4.0;

    group.bench_function("Baseline", |b| {
        b.iter(|| apply_heat_vision_baseline(black_box(depth), black_box(min_z), black_box(range)));
    });

    group.bench_function("FMA Optimized", |b| {
        b.iter(|| apply_heat_vision_fma(black_box(depth), black_box(min_z), black_box(range)));
    });

    group.finish();
}

criterion_group!(benches, bench_fma_vs_baseline);
criterion_main!(benches);
