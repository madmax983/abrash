use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_zbuffer_clear_rect_extreme(c: &mut Criterion) {
    let mut zb = ZBuffer::new(3840, 2160).unwrap();
    c.bench_function("zbuffer/clear_rect_extreme", |b| {
        b.iter(|| {
            zb.clear_rect(
                black_box(10),
                black_box(10),
                black_box(3800),
                black_box(2140),
            );
        });
    });
}

criterion_group!(benches, bench_zbuffer_clear_rect_extreme);
criterion_main!(benches);
