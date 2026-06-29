use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::blueprint::{BlueprintConfig, apply_blueprint};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_blueprint(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = BlueprintConfig::default();

    c.bench_function("blueprint_800x600", |b| {
        b.iter(|| {
            apply_blueprint(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_blueprint);
criterion_main!(benches);
