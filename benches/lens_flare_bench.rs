use abrash::experimental::lens_flare::{apply_lens_flare, LensFlareConfig};
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_lens_flare(c: &mut Criterion) {
    c.bench_function("apply_lens_flare 1080p", |b| {
        // Setup a 1080p framebuffer
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        let config = LensFlareConfig::default();
        let light_pos = Vec2::new(960.0, 540.0);

        b.iter(|| {
            apply_lens_flare(&mut fb, black_box(light_pos), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_lens_flare);
criterion_main!(benches);
