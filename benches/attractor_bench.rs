use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::attractors::{AttractorType, StrangeAttractor};
use criterion::{Criterion, criterion_group, criterion_main};

fn attractor_bench(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let attractor = StrangeAttractor {
        attractor_type: AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        },
        iterations: 100_000,
        ..Default::default()
    };

    c.bench_function("strange_attractor_lorenz_100k", |b| {
        b.iter(|| {
            fb.clear(0xFF_00_00_00);
            attractor.render(&mut fb);
        });
    });
}

criterion_group!(benches, attractor_bench);
criterion_main!(benches);
