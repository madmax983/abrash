use abrash::experimental::frosted_glass::apply_frosted_glass;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_frosted_glass(c: &mut Criterion) {
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

    c.bench_function("apply_frosted_glass 1080p (radius=8)", |b| {
        b.iter(|| {
            apply_frosted_glass(black_box(&mut fb), black_box(8.0), black_box(42));
        });
    });
}

criterion_group!(benches, benchmark_frosted_glass);
criterion_main!(benches);
