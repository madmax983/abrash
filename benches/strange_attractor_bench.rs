use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::strange_attractor::{render_strange_attractor, AttractorConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn strange_attractor_bench(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = AttractorConfig {
        iterations: 100_000,
        ..Default::default()
    }; // use a smaller number for benchmarking

    c.bench_function("strange_attractor 100k", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            render_strange_attractor(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, strange_attractor_bench);
criterion_main!(benches);
