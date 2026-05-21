use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::lens_flare::{LensFlareConfig, apply_lens_flare};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_lens_flare(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = LensFlareConfig::default();
    let light_pos = Vec2::new(400.0, 300.0);

    c.bench_function("Lens Flare/800x600", |b| {
        b.iter(|| apply_lens_flare(black_box(&mut fb), black_box(light_pos), black_box(&config)));
    });
}

criterion_group!(benches, bench_lens_flare);
criterion_main!(benches);
