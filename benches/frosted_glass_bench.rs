use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::frosted_glass::apply_frosted_glass;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_frosted_glass(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let scatter_radius = 5;

    // Warmup frame to fill it with some data
    fb.clear(0xFF202020);
    for x in 0..1920 {
        for y in 0..1080 {
            if (x / 20) % 2 == (y / 20) % 2 {
                fb.set_pixel(x, y, 0xFFFFFFFF);
            }
        }
    }

    c.bench_function("frosted_glass_1920x1080", |b| {
        b.iter(|| {
            apply_frosted_glass(&mut fb, scatter_radius, 0);
        });
    });
}

criterion_group!(benches, bench_frosted_glass);
criterion_main!(benches);
