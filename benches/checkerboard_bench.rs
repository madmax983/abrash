use abrash_render::procedural::checkerboard;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_checkerboard(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkerboard_procedural");

    group.bench_function("128x128", |b| {
        b.iter(|| {
            checkerboard(
                black_box(128),
                black_box(128),
                black_box(16),
                black_box(0xFFFFFFFF),
                black_box(0xFF000000),
            )
        });
    });

    group.finish();
}

criterion_group!(benches, bench_checkerboard);
criterion_main!(benches);
