use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::precipitation::{PrecipitationConfig, apply_precipitation};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_precipitation_spawn(c: &mut Criterion) {
    // Large number of drops to measure initialization/spawn overhead
    let config = PrecipitationConfig {
        max_drops: 50_000,
        ..Default::default()
    };

    let mut fb = Framebuffer::new(800, 600).unwrap();
    let zb = ZBuffer::new(800, 600).unwrap();

    c.bench_function("precipitation_spawn_50k", |b| {
        b.iter(|| {
            apply_precipitation(&mut fb, &zb, &config);
        });
    });
}

criterion_group!(benches, bench_precipitation_spawn);
criterion_main!(benches);
