use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::hologram::{HologramConfig, apply_hologram};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_hologram(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = HologramConfig::default();

    c.bench_function("hologram_1080p", |b| {
        b.iter(|| apply_hologram(black_box(&mut fb), black_box(&config)));
    });
}

criterion_group!(benches, benchmark_hologram);
criterion_main!(benches);
