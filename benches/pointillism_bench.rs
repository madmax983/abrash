use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_pointillism(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let src_fb = Framebuffer::new(800, 600).unwrap();
    let config = PointillismConfig::default();

    c.bench_function("pointillism_800x600", |b| {
        b.iter(|| {
            apply_pointillism(&mut fb, &src_fb, &config);
        });
    });
}

criterion_group!(benches, bench_pointillism);
criterion_main!(benches);
