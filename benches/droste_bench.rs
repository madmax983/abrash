use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::droste::{DrosteConfig, apply_droste};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_droste(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Fill with some dummy data
    for y in 0..600 {
        for x in 0..800 {
            let color = 0xFF00_0000 | (x as u32 & 0xFF) << 16 | (y as u32 & 0xFF) << 8;
            fb.set_pixel(x, y, color);
        }
    }

    let config = DrosteConfig {
        iterations: 4,
        scale: 0.5,
        offset_x: 0.0,
        offset_y: 0.0,
    };

    c.bench_function("apply_droste 800x600", |b| {
        b.iter(|| apply_droste(&mut fb, &config))
    });
}

criterion_group!(benches, bench_droste);
criterion_main!(benches);
