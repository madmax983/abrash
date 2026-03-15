use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::PI;

#[cfg(feature = "nova")]
use abrash::experimental::swirl::{SwirlConfig, apply_swirl};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
fn bench_apply_swirl(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_FF_00_FF); // Fill with magenta

    let config = SwirlConfig {
        center_x: 0.5,
        center_y: 0.5,
        radius: 400.0,
        angle: PI,
    };

    c.bench_function("apply_swirl_1080p", |b| {
        b.iter(|| {
            apply_swirl(black_box(&mut fb), black_box(&config));
        })
    });
}

#[cfg(not(feature = "nova"))]
fn bench_apply_swirl(_c: &mut Criterion) {}

criterion_group!(benches, bench_apply_swirl);
criterion_main!(benches);
