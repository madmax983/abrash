use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::strange_attractor::{StrangeAttractor, AttractorType};

fn bench_strange_attractor_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("StrangeAttractor");

    let num_particles = 100_000;

    group.bench_function("step_100k", |b| {
        let mut sys = StrangeAttractor::new(num_particles, AttractorType::Lorenz { sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0 }, 0.01);
        b.iter(|| {
            sys.step();
            black_box(&sys);
        });
    });

    group.bench_function("render_100k", |b| {
        let sys = StrangeAttractor::new(num_particles, AttractorType::Lorenz { sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0 }, 0.01);
        let mut fb = Framebuffer::new(800, 600).unwrap();
        b.iter(|| {
            sys.render(&mut fb, 10.0, 400.0, 300.0);
            black_box(&fb);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_strange_attractor_step);
criterion_main!(benches);
