use abrash::framebuffer::Framebuffer;
use abrash::post_process::{apply_grayscale, apply_scanlines};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_grayscale(c: &mut Criterion) {
    let mut framebuffer = Framebuffer::new(800, 600).unwrap();
    // Fill with random noise or a pattern
    for i in 0..800 * 600 {
        framebuffer.as_mut_slice()[i as usize] = i as u32;
    }

    c.bench_function("post_process_grayscale_800x600", |b| {
        b.iter(|| apply_grayscale(black_box(&mut framebuffer)))
    });
}

fn benchmark_scanlines(c: &mut Criterion) {
    let mut framebuffer = Framebuffer::new(800, 600).unwrap();
    // Fill with random noise or a pattern
    for i in 0..800 * 600 {
        framebuffer.as_mut_slice()[i as usize] = i as u32;
    }

    c.bench_function("post_process_scanlines_800x600", |b| {
        b.iter(|| apply_scanlines(black_box(&mut framebuffer)))
    });
}

criterion_group!(benches, benchmark_grayscale, benchmark_scanlines);
criterion_main!(benches);
