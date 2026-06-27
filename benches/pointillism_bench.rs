use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn pointillism_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_AA_BB_CC);
    let config = PointillismConfig::default();

    c.bench_function("apply_pointillism 800x600", |b| {
        b.iter(|| {
            apply_pointillism(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, pointillism_benchmark);
criterion_main!(benches);
