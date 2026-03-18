use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::fisheye::{FisheyeConfig, apply_fisheye};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
fn bench_apply_fisheye(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_FF_00_FF); // Fill with magenta

    let config = FisheyeConfig {
        center_x: 0.5,
        center_y: 0.5,
        radius: 400.0,
        strength: 1.5,
    };

    c.bench_function("apply_fisheye_1080p", |b| {
        b.iter(|| {
            apply_fisheye(black_box(&mut fb), black_box(&config));
        });
    });
}

#[cfg(not(feature = "nova"))]
fn bench_apply_fisheye(_c: &mut Criterion) {}

criterion_group!(benches, bench_apply_fisheye);
criterion_main!(benches);
