#![cfg(feature = "nova")]

use abrash::experimental::lens_flare::{apply_lens_flare, LensFlareConfig};
use abrash::framebuffer::Framebuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_lens_flare(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF101010); // Very dark gray

    // Place a very bright spot (e.g. sun) in the scene
    for dy in -10..=10 {
        for dx in -10..=10 {
            fb.set_pixel(200 + dx, 150 + dy, 0xFFFFFFFF);
        }
    }

    let config = LensFlareConfig::default();

    c.bench_function("apply_lens_flare_800x600", |b| {
        b.iter(|| apply_lens_flare(black_box(&mut fb), black_box(&config), 200.0, 150.0))
    });
}

criterion_group!(benches, bench_lens_flare);
criterion_main!(benches);
