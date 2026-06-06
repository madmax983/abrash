use abrash::experimental::metaballs::{Metaballs, MetaballsConfig};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_metaballs(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut config = MetaballsConfig::default();
    config.num_balls = 50;
    let mut scene = Metaballs::new(config);

    c.bench_function("render_metaballs_800x600_50", |b| {
        b.iter(|| {
            scene.update_and_render(black_box(&mut fb));
        });
    });
}

criterion_group!(benches, bench_metaballs);
criterion_main!(benches);
