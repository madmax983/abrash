use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_framebuffer_clear_rect(c: &mut Criterion) {
    let mut group = c.benchmark_group("framebuffer_clear_rect");
    group.sample_size(10);

    // Test a common resolution, e.g., 1920x1080
    let mut fb = Framebuffer::new(1920, 1080).unwrap();

    // Clear a 500x500 rect in the center
    group.bench_function("clear_500x500", |b| {
        b.iter(|| {
            fb.clear_rect(
                black_box(710),
                black_box(290),
                black_box(500),
                black_box(500),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    // Clear the full screen using clear_rect
    group.bench_function("clear_fullscreen", |b| {
        b.iter(|| {
            fb.clear_rect(
                black_box(0),
                black_box(0),
                black_box(1920),
                black_box(1080),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    group.finish();
}

fn bench_zbuffer_clear_rect(c: &mut Criterion) {
    let mut group = c.benchmark_group("zbuffer_clear_rect");
    group.sample_size(10);

    let mut zb = ZBuffer::new(1920, 1080).unwrap();

    group.bench_function("clear_500x500", |b| {
        b.iter(|| {
            zb.clear_rect(
                black_box(710),
                black_box(290),
                black_box(500),
                black_box(500),
            );
        });
    });

    group.bench_function("clear_fullscreen", |b| {
        b.iter(|| {
            zb.clear_rect(black_box(0), black_box(0), black_box(1920), black_box(1080));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_framebuffer_clear_rect,
    bench_zbuffer_clear_rect
);
criterion_main!(benches);
