#![cfg(feature = "nova")]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::synthwave::{SynthwaveConfig, apply_synthwave};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn synthwave_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = SynthwaveConfig::default();

    c.bench_function("synthwave_800x600", |b| {
        b.iter(|| {
            apply_synthwave(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, synthwave_benchmark);
criterion_main!(benches);
