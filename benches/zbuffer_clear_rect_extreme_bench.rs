use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_zbuffer_clear_rect_extreme(c: &mut Criterion) {
    let mut group = c.benchmark_group("zbuffer_extreme");
    group.sample_size(100);

    let mut zb_1080 = ZBuffer::new(1920, 1080).unwrap();

    group.bench_function("clear_rect_extreme_1080p", |b| {
        b.iter(|| {
            zb_1080.clear_rect(
                black_box(-5000),
                black_box(-5000),
                black_box(u32::MAX),
                black_box(u32::MAX),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_zbuffer_clear_rect_extreme);
criterion_main!(benches);
