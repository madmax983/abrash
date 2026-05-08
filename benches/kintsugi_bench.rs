#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::kintsugi::{KintsugiConfig, apply_kintsugi};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_kintsugi(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a pattern that creates edges
    for y in 0..height {
        for x in 0..width {
            if (x / 50 + y / 50) % 2 == 0 {
                fb.set_pixel(x as i32, y as i32, 0xFF_FF_FF_FF);
            } else {
                fb.set_pixel(x as i32, y as i32, 0xFF_00_00_00);
            }
        }
    }

    let config = KintsugiConfig::default();

    c.bench_function("kintsugi_800x600", |b| {
        b.iter(|| {
            apply_kintsugi(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_kintsugi);
criterion_main!(benches);
