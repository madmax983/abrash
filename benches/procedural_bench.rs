use abrash_render::procedural::{white_noise, xor_pattern, grid_pattern};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_procedural(c: &mut Criterion) {
    let mut group = c.benchmark_group("Procedural");
    group.bench_function("white_noise_256", |b| {
        b.iter(|| black_box(white_noise(256, 256, 12345).unwrap()));
    });
    group.bench_function("xor_pattern_256", |b| {
        b.iter(|| black_box(xor_pattern(256, 256).unwrap()));
    });
    group.bench_function("grid_pattern_256", |b| {
        b.iter(|| black_box(grid_pattern(256, 256, 16, 0xFFFFFFFF, 0xFF000000).unwrap()));
    });
    group.finish();
}

criterion_group!(benches, bench_procedural);
criterion_main!(benches);
