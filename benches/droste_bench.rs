use abrash::experimental::droste::{DrosteConfig, apply_droste};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_droste(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = DrosteConfig::default();

    c.bench_function("droste_filter", |b| {
        b.iter(|| apply_droste(black_box(&mut fb), black_box(&config)))
    });
}

criterion_group!(benches, bench_droste);
criterion_main!(benches);
