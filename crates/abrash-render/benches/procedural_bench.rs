use abrash_render::procedural::{grid_pattern, white_noise, xor_pattern};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_procedural(c: &mut Criterion) {
    let mut group = c.benchmark_group("Procedural Textures (2048x2048)");

    group.bench_function("xor_pattern", |b| {
        b.iter(|| {
            black_box(xor_pattern(2048, 2048).unwrap());
        });
    });

    group.bench_function("grid_pattern", |b| {
        b.iter(|| {
            black_box(grid_pattern(2048, 2048, 16, 0xFFFF_FFFF, 0xFF00_0000).unwrap());
        });
    });

    group.bench_function("white_noise", |b| {
        b.iter(|| {
            black_box(white_noise(2048, 2048, 12345).unwrap());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_procedural);
criterion_main!(benches);
