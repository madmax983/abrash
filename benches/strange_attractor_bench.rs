use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::strange_attractor::{AttractorConfig, render_attractor};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn strange_attractor_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("strange_attractor");

    // Test on a standard 1080p equivalent frame and high iteration count
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = AttractorConfig {
        iterations: 1_000_000,
        ..Default::default()
    };

    group.bench_function("peter_de_jong_1m", |b| {
        b.iter(|| {
            fb.clear(0);
            render_attractor(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

criterion_group!(benches, strange_attractor_benchmark);
criterion_main!(benches);
