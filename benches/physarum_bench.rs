#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::physarum::{PhysarumConfig, apply_physarum};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_physarum(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = PhysarumConfig::default();

    c.bench_function("physarum_step", |b| {
        b.iter(|| {
            apply_physarum(black_box(&mut fb), black_box(&config));
        });

    });
}

criterion_group!(benches, bench_physarum);
criterion_main!(benches);
