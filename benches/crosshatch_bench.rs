use abrash_render::experimental::crosshatch::apply_crosshatch;
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_crosshatch(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x & 0xFF) << 16 | (y & 0xFF) << 8;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_crosshatch 1080p (spacing=5)", |b| {
        b.iter(|| {
            apply_crosshatch(black_box(&mut fb), black_box(5));
        });
    });

    c.bench_function("apply_crosshatch 1080p (spacing=10)", |b| {
        b.iter(|| {
            apply_crosshatch(black_box(&mut fb), black_box(10));
        });
    });
}

criterion_group!(benches, benchmark_crosshatch);
criterion_main!(benches);
