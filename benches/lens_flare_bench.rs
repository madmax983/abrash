use abrash::experimental::lens_flare::{apply_lens_flare, LensFlareConfig};
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use criterion::{criterion_group, criterion_main, Criterion};

pub fn bench_lens_flare(c: &mut Criterion) {
    let mut group = c.benchmark_group("lens_flare");

    let widths = [1920];
    let heights = [1080];

    for (&width, &height) in widths.iter().zip(heights.iter()) {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let config = LensFlareConfig::default();
        let light_pos = Vec2::new(width as f32 * 0.8, height as f32 * 0.2);

        group.bench_function(format!("apply_lens_flare/{}x{}", width, height), |b| {
            b.iter(|| apply_lens_flare(&mut fb, light_pos, &config))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_lens_flare);
criterion_main!(benches);
