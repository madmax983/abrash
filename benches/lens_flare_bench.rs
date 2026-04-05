use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::lens_flare::{apply_lens_flare, LensFlareConfig};

fn bench_lens_flare(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = LensFlareConfig::default();
    let light_pos = Vec2::new(500.0, 500.0);

    c.bench_function("apply_lens_flare 1080p", |b| {
        b.iter(|| apply_lens_flare(black_box(&mut fb), black_box(light_pos), black_box(&config)))
    });
}

criterion_group!(benches, bench_lens_flare);
criterion_main!(benches);
