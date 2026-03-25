use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_clear_rect_1080p(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    c.bench_function("framebuffer/clear_rect_1080p", |b| {
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
}

fn bench_clear_rect_4k(c: &mut Criterion) {
    let mut fb = Framebuffer::new(3840, 2160).unwrap();
    c.bench_function("framebuffer/clear_rect_4k", |b| {
        b.iter(|| {
            fb.clear_rect(
                black_box(500),
                black_box(500),
                black_box(2000),
                black_box(1000),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

fn bench_zbuffer_clear_rect_1080p(c: &mut Criterion) {
    let mut zb = ZBuffer::new(1920, 1080).unwrap();
    c.bench_function("zbuffer/clear_rect_1080p", |b| {
        b.iter(|| {
            zb.clear_rect(
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
            );
        });
    });
}

fn bench_zbuffer_clear_rect_4k(c: &mut Criterion) {
    let mut zb = ZBuffer::new(3840, 2160).unwrap();
    c.bench_function("zbuffer/clear_rect_4k", |b| {
        b.iter(|| {
            zb.clear_rect(
                black_box(500),
                black_box(500),
                black_box(2000),
                black_box(1000),
            );
        });
    });
}


criterion_group!(benches, bench_clear_rect_1080p, bench_clear_rect_4k, bench_zbuffer_clear_rect_1080p, bench_zbuffer_clear_rect_4k);
criterion_main!(benches);
