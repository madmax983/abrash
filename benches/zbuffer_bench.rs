use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_zbuffer_clear(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut zb = ZBuffer::new(width, height).unwrap();

    let mut group = c.benchmark_group("zbuffer");

    group.bench_function("clear_1080p", |b| {
        b.iter(|| {
            zb.clear();
            black_box(&zb);
        });
    });

    let width_4k = 3840;
    let height_4k = 2160;
    let mut zb_4k = ZBuffer::new(width_4k, height_4k).unwrap();

    group.bench_function("clear_4k", |b| {
        b.iter(|| {
            zb_4k.clear();
            black_box(&zb_4k);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_zbuffer_clear);
criterion_main!(benches);
