use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::cel_shade::{CelShadeConfig, apply_cel_shade};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_cel_shade(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    // Fill buffers with some data
    for y in 0..600 {
        for x in 0..800 {
            fb.set_pixel(x, y, 0xFF_123456 + (x as u32) + (y as u32));
            zb.test_and_set(x, y, 0.5);
        }
    }

    let config = CelShadeConfig {
        levels: 4,
        edge_threshold: 0.2,
        edge_color: 0xFF_000000,
    };

    c.bench_function("cel_shade_800x600", |b| {
        b.iter(|| {
            apply_cel_shade(black_box(&mut fb), black_box(&zb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_cel_shade);
criterion_main!(benches);
