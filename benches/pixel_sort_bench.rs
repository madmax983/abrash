use abrash::experimental::pixel_sort::{apply_pixel_sort, PixelSortConfig};
use abrash::framebuffer::Framebuffer;
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Fill with some gradient to sort
    for y in 0..1080 {
        for x in 0..1920 {
            let lum = ((x + y) % 256) as u32;
            let color = 0xFF00_0000 | (lum << 16) | (lum << 8) | lum;
            unsafe {
                fb.set_pixel_unchecked(x, y, color);
            }
        }
    }

    let config = PixelSortConfig {
        threshold: 0.5,
        vertical: false,
        reverse: false,
    };

    c.bench_function("apply_pixel_sort 1080p", |b| {
        b.iter(|| {
            apply_pixel_sort(&mut fb, &config);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
