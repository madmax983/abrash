use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::wobble::{WobbleConfig, apply_wobble};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_wobble(c: &mut Criterion) {
    let mut group = c.benchmark_group("wobble");
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let config = WobbleConfig {
        amplitude: 10.0,
        frequency: 5.0,
        time: 0.0,
    };

    group.bench_function("apply_wobble_1080p", |b| {
        b.iter(|| {
            apply_wobble(&mut fb, &config);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_wobble);
criterion_main!(benches);
