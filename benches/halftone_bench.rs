use abrash::experimental::halftone::apply_halftone;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::PI;

fn benchmark_halftone(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | ((x & 0xFF) << 16) | ((y & 0xFF) << 8);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_halftone 1080p (dot_size=4.0, angle=PI/4)", |b| {
        b.iter(|| {
            apply_halftone(black_box(&mut fb), black_box(4.0), black_box(PI / 4.0));
        });
    });
}

criterion_group!(benches, benchmark_halftone);
criterion_main!(benches);
