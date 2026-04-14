use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_zbuffer_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("zbuffer");
    group.sample_size(100);

    let mut zb_1080 = ZBuffer::new(1920, 1080).unwrap();
    let mut zb_4k = ZBuffer::new(3840, 2160).unwrap();

    group.bench_function("clear_1080p", |b| {
        b.iter(|| zb_1080.clear());
    });

    group.bench_function("clear_4k", |b| {
        b.iter(|| zb_4k.clear());
    });

    group.bench_function("clear_rect_1080p", |b| {
        b.iter(|| zb_1080.clear_rect(100, 100, 1000, 500));
    });

    group.bench_function("clear_rect_4k", |b| {
        b.iter(|| zb_4k.clear_rect(200, 200, 2000, 1000));
    });

    group.finish();
}

criterion_group!(benches, bench_zbuffer_clear);
criterion_main!(benches);
