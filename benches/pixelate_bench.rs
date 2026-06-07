use abrash_render::experimental::pixelate::apply_pixelate;
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_pixelate(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern to ensure memory is somewhat dirty
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | ((x & 0xFF) << 16) | ((y & 0xFF) << 8);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_pixelate 1080p (block_size=8)", |b| {
        b.iter(|| {
            apply_pixelate(
                black_box(&mut fb),
                black_box(&abrash_render::experimental::pixelate::PixelateConfig { block_size: 8 }),
            );
        });
    });

    c.bench_function("apply_pixelate 1080p (block_size=16)", |b| {
        b.iter(|| {
            apply_pixelate(
                black_box(&mut fb),
                black_box(&abrash_render::experimental::pixelate::PixelateConfig {
                    block_size: 16,
                }),
            );
        });
    });
}

criterion_group!(benches, benchmark_pixelate);
criterion_main!(benches);
