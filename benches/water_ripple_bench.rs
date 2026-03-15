#![cfg(feature = "nova")]

use abrash::experimental::water_ripple::{RippleConfig, apply_water_ripple};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_water_ripple(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill the framebuffer with a simple gradient pattern
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x << 16) | (y << 8);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config = RippleConfig {
        center_x: 0.5,
        center_y: 0.5,
        amplitude: 15.0,
        frequency: 30.0,
        phase: 1.0,
        radius: 0.6,
    };

    c.bench_function("water_ripple_800x600", |b| {
        b.iter(|| {
            apply_water_ripple(black_box(&mut fb), black_box(config));
        });
    });
}

criterion_group!(benches, bench_water_ripple);
criterion_main!(benches);
