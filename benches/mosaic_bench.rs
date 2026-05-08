use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::mosaic::apply_hex_mosaic;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_mosaic(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x % 256) << 16 | (y % 256) << 8 | ((x + y) % 256);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("mosaic_800x600", |b| {
        b.iter(|| {
            apply_hex_mosaic(
                black_box(&mut fb),
                black_box(10.0),
                black_box(1.0),
                black_box(0xFF00_0000),
            );
        });
    });
}

criterion_group!(benches, bench_mosaic);
criterion_main!(benches);
