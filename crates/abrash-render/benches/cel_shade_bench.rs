use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::cel_shade::{apply_cel_shade, CelShadeConfig};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_cel_shade(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cel Shade");
    let (w, h) = (800, 600);
    let mut fb = Framebuffer::new(w, h).unwrap();
    let mut zb = ZBuffer::new(w, h).unwrap();

    // Fill with a dummy gradient instead of using rand
    for y in 0..h {
        for x in 0..w {
            let color = 0xFF00_0000 | (x % 255) | ((y % 255) << 8);
            fb.set_pixel(x as i32, y as i32, color);
            let depth = (x as f32) / (w as f32) * 100.0;
            unsafe { zb.test_and_set_unchecked(x as usize, y as usize, depth) };
        }
    }

    let config = CelShadeConfig::default();
    group.bench_function("800x600", |b| {
        b.iter(|| {
            apply_cel_shade(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });
    group.finish();
}
criterion_group!(benches, bench_cel_shade);
criterion_main!(benches);
