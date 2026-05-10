use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_pointillism(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_AA_BB_CC);
    let config = PointillismConfig::default();

    c.bench_function("apply_pointillism 1080p", |b| {
        b.iter(|| {
            apply_pointillism(&mut fb, &config);
        });
    });
}

criterion_group!(benches, bench_pointillism);
criterion_main!(benches);
