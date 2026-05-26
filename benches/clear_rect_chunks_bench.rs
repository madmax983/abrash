use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_clear_rect_chunks(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear_rect_chunks");
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut zb = ZBuffer::new(1920, 1080).unwrap();

    // The dimensions trigger the chunking fallback path because
    // it starts from x=100 and ends at x=1100 (which is not full width 1920)
    group.bench_function("framebuffer", |b| {
        b.iter(|| {
            fb.clear_rect(
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    group.bench_function("zbuffer", |b| {
        b.iter(|| {
            zb.clear_rect(
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clear_rect_chunks);
criterion_main!(benches);
