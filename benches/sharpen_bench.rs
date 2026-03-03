use abrash::experimental::sharpen::apply_sharpen;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_sharpen(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern to ensure memory is somewhat dirty
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF000000 | (x & 0xFF) << 16 | (y & 0xFF) << 8;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_sharpen 1080p", |b| {
        b.iter(|| {
            apply_sharpen(black_box(&mut fb), black_box(1.0));
        });
    });
}

criterion_group!(benches, benchmark_sharpen);
criterion_main!(benches);
