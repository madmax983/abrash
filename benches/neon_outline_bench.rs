use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::neon_outline::{NeonOutlineConfig, apply_neon_outline};
use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;

fn bench_neon_outline(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with random noise to ensure worst-case edge detection
    let mut rng = rand::thread_rng();
    for y in 0..height {
        for x in 0..width {
            let col = if rng.gen_bool(0.1) {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, col);
        }
    }

    let config = NeonOutlineConfig::default();

    c.bench_function("neon_outline_640x480", |b| {
        b.iter(|| {
            apply_neon_outline(&mut fb, &config);
        });
    });
}

criterion_group!(benches, bench_neon_outline);
criterion_main!(benches);
