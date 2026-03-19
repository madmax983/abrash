use abrash::experimental::tilt_shift::{TiltShiftConfig, apply_tilt_shift};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_tilt_shift(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let config = TiltShiftConfig {
        focus_dist: 0.5,
        focus_range: 0.2,
        blur_radius: 10,
    };

    c.bench_function("tilt_shift_1080p", |b| {
        b.iter(|| {
            apply_tilt_shift(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_tilt_shift);
criterion_main!(benches);
