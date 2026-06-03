use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::metaballs::{Metaballs, MetaballsConfig};
use criterion::{Criterion, criterion_group, criterion_main};

fn metaballs_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut sim = Metaballs::new(MetaballsConfig::default());

    c.bench_function("metaballs_update_and_render_800x600", |b| {
        b.iter(|| {
            sim.update_and_render(&mut fb);
            std::hint::black_box(&fb);
        });
    });
}

criterion_group!(benches, metaballs_benchmark);
criterion_main!(benches);
