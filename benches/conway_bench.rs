use abrash::experimental::conway::{ConwayConfig, apply_conway};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_apply_conway(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Pre-seed some pixels so there's work to do
    for y in 0..1080 {
        for x in 0..1920 {
            if (x + y) % 5 == 0 {
                fb.set_pixel(x, y, 0xFF_FFFFFF);
            }
        }
    }

    let config = ConwayConfig {
        cell_size: 4,
        seed_threshold: 128,
        live_color: 0xFF_00FF00,
        dead_color: 0xFF_000000,
        overlay: false,
        overlay_opacity: 1.0,
    };

    let mut group = c.benchmark_group("conway");
    group.bench_function("apply_conway_1080p", |b| {
        b.iter(|| {
            apply_conway(black_box(&mut fb), black_box(&config));
        });
    });
    group.finish();
}

criterion_group!(benches, bench_apply_conway);
criterion_main!(benches);
