use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::post_process;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::PI;

fn benchmark_grayscale(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with a pattern
    fb.clear(0xFFFF0000);

    c.bench_function("apply_grayscale 1080p", |b| {
        b.iter(|| {
            post_process::apply_grayscale(black_box(&mut fb));
        })
    });
}

fn benchmark_scanlines(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFFFFFF);

    c.bench_function("apply_scanlines 1080p", |b| {
        b.iter(|| {
            post_process::apply_scanlines(black_box(&mut fb));
        })
    });
}

fn benchmark_invert(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF000000);

    c.bench_function("apply_invert 1080p", |b| {
        b.iter(|| {
            post_process::apply_invert(black_box(&mut fb));
        })
    });
}

fn benchmark_sepia(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFFFFFF); // White

    c.bench_function("apply_sepia 1080p", |b| {
        b.iter(|| {
            post_process::apply_sepia(black_box(&mut fb));
        })
    });
}

fn benchmark_chromatic_aberration(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFFFFFF); // White

    c.bench_function("apply_chromatic_aberration 1080p", |b| {
        b.iter(|| {
            post_process::apply_chromatic_aberration(black_box(&mut fb), black_box(5));
        })
    });
}

fn benchmark_bloom(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFFFFFF); // White

    c.bench_function("apply_bloom 1080p (r=10)", |b| {
        b.iter(|| {
            post_process::apply_bloom(
                black_box(&mut fb),
                black_box(200),
                black_box(10),
                black_box(0.8),
            );
        })
    });
}

fn benchmark_ssao(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);

    // Populate buffers
    fb.clear(0xFFFFFFFF);
    // Fill Z buffer with some data (gradient)
    for y in 0..height {
        for x in 0..width {
            let depth = 0.5 + (x as f32 / width as f32) * 0.4;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    c.bench_function("apply_ssao 1080p", |b| {
        b.iter(|| {
            post_process::apply_ssao(
                black_box(&mut fb),
                black_box(&zb),
                black_box(&proj),
                black_box(1.0),
                black_box(0.001),
                black_box(2.0),
            );
        })
    });
}

criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration,
    benchmark_bloom,
    benchmark_ssao,
);
criterion_main!(benches);
