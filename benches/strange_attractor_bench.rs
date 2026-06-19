use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::strange_attractor::{
    AttractorType, StrangeAttractor, StrangeAttractorConfig,
};

#[cfg(feature = "nova")]
fn bench_strange_attractor(c: &mut Criterion) {
    let mut group = c.benchmark_group("Strange Attractor Renderer");
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut attractor = StrangeAttractor::new(800, 600);
    // Use a smaller number of iterations for the benchmark to keep runs fast,
    // while still doing enough work to measure the hot loop reliably.
    let mut config = StrangeAttractorConfig {
        iterations: 1_000_000,
        ..Default::default()
    };

    group.bench_function("render_clifford", |b| {
        b.iter(|| {
            attractor.render(black_box(&mut fb), black_box(&config));
        });
    });

    config.attractor_type = AttractorType::PeterDeJong;
    config.a = 1.641;
    config.b = 1.902;
    config.c = 0.316;
    config.d = 1.525;

    group.bench_function("render_peter_de_jong", |b| {
        b.iter(|| {
            attractor.render(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_strange_attractor(_c: &mut Criterion) {
    // Benchmark does nothing when the feature is disabled
}

criterion_group!(benches, bench_strange_attractor);
criterion_main!(benches);
