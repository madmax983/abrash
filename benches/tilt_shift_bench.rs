use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::tilt_shift::{TiltShiftConfig, apply_tilt_shift};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
fn bench_apply_tilt_shift(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();

    // Fill with a simple pattern
    for y in 0..1080 {
        for x in 0..1920 {
            let r = (x % 256) as u32;
            let g = (y % 256) as u32;
            let b = ((x + y) % 256) as u32;
            fb.set_pixel(x, y, 0xFF00_0000 | (r << 16) | (g << 8) | b);
        }
    }

    let config = TiltShiftConfig {
        focus_dist: 0.5,
        focus_range: 0.2,
        blur_radius: 5,
    };

    c.bench_function("apply_tilt_shift_1080p", |b| {
        b.iter(|| {
            apply_tilt_shift(black_box(&mut fb), black_box(&config));
        });
    });
}

#[cfg(not(feature = "nova"))]
fn bench_apply_tilt_shift(_c: &mut Criterion) {}

criterion_group!(benches, bench_apply_tilt_shift);
criterion_main!(benches);
