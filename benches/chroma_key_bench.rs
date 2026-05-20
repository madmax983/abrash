use abrash::experimental::chroma_key::{apply_chroma_key, smooth_chroma_key};
use abrash_core::framebuffer::Framebuffer;
use criterion::{criterion_group, criterion_main, Criterion};

fn chroma_key_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chroma Key");

    // Create 1080p framebuffers
    let width = 1920;
    let height = 1080;

    let mut fg = Framebuffer::new(width as u32, height as u32).unwrap();
    let mut bg = Framebuffer::new(width as u32, height as u32).unwrap();

    // Fill background with white
    bg.clear(0xFF_FFFFFF);

    // Fill foreground with a mix of colors to test different branches
    for y in 0..height {
        for x in 0..width {
            let color = if x < width / 3 {
                0xFF_00FF00 // Exact match
            } else if x < (2 * width) / 3 {
                0xFF_00AA00 // Needs feathering
            } else {
                0xFF_FF0000 // Completely different
            };
            fg.set_pixel(x as i32, y as i32, color);
        }
    }

    let key_color = 0xFF_00FF00;
    let threshold = 20.0;
    let feather = 100.0;

    group.bench_function("apply_chroma_key/1080p", |b| {
        b.iter(|| apply_chroma_key(&mut fg, &bg, key_color))
    });

    group.bench_function("smooth_chroma_key/1080p", |b| {
        b.iter(|| smooth_chroma_key(&mut fg, &bg, key_color, threshold, feather))
    });

    group.finish();
}

criterion_group!(benches, chroma_key_benchmark);
criterion_main!(benches);
