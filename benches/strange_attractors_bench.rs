use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::strange_attractors::{
    StrangeAttractorsConfig, apply_strange_attractors,
};
use criterion::{Criterion, criterion_group, criterion_main};

fn benchmark_strange_attractors(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = StrangeAttractorsConfig::default();

    c.bench_function("strange_attractors_800x600_1m_iters", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            apply_strange_attractors(&mut fb, &config);
        });
    });
}

criterion_group!(benches, benchmark_strange_attractors);
criterion_main!(benches);
