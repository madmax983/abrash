use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::cel_shade::{CelShadeConfig, apply_cel_shade};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_cel_shade(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill with some gradients to prevent branch prediction from being too perfect
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x % 256) << 16 | (y % 256) << 8 | ((x + y) % 256);
            fb.set_pixel(x as i32, y as i32, color);
            zb.test_and_set(x as i32, y as i32, (x as f32) / (width as f32));
        }
    }

    let config = CelShadeConfig::default();

    c.bench_function("cel_shade_800x600", |b| {
        b.iter(|| {
            apply_cel_shade(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_cel_shade);
criterion_main!(benches);
