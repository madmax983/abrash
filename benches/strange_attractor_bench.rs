use abrash_core::math::Vec3;
use abrash_render::experimental::strange_attractor::{
    render_strange_attractor, AttractorConfig, AttractorType,
};
use abrash_core::framebuffer::Framebuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn strange_attractor_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = AttractorConfig {
        attractor_type: AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        },
        iterations: 100_000,
        dt: 0.01,
        scale: 10.0,
        start_pos: Vec3::new(0.1, 0.0, 0.0),
        color: 0xFF_00FF00,
        additive: true,
    };

    c.bench_function("strange_attractor_100k", |b| {
        b.iter(|| render_strange_attractor(black_box(&mut fb), black_box(&config)))
    });
}

criterion_group!(benches, strange_attractor_benchmark);
criterion_main!(benches);
