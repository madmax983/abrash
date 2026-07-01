use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_pointillism(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_FF_00_00); // Red background

    let config = PointillismConfig::default();

    c.bench_function("pointillism_filter", |b| {
        b.iter(|| {
            apply_pointillism(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, benchmark_pointillism);
criterion_main!(benches);
