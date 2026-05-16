use abrash::experimental::crt::apply_crt;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_crt(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x & 0xFF) << 16 | (y & 0xFF) << 8;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_crt 1080p (distortion=0.2)", |b| {
        b.iter(|| {
            apply_crt(black_box(&mut fb), black_box(0.2));
        });
    });

    c.bench_function("apply_crt 1080p (distortion=0.5)", |b| {
        b.iter(|| {
            apply_crt(black_box(&mut fb), black_box(0.5));
        });
    });

    c.bench_function("apply_crt 1080p (distortion=0.0)", |b| {
        b.iter(|| {
            apply_crt(black_box(&mut fb), black_box(0.0));
        });
    });

    let mut fb_4k = Framebuffer::new(3840, 2160).unwrap();
    for y in 0..2160 {
        for x in 0..3840 {
            let color = 0xFF00_0000 | (x & 0xFF) << 16 | (y & 0xFF) << 8;
            fb_4k.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_crt 4k (distortion=0.2)", |b| {
        b.iter(|| {
            apply_crt(black_box(&mut fb_4k), black_box(0.2));
        });
    });
}

criterion_group!(benches, benchmark_crt);
criterion_main!(benches);
