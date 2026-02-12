use abrash::framebuffer::Framebuffer;
use abrash::post_process;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

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

criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration
);
criterion_main!(benches);
