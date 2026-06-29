use abrash::experimental::pointillism::apply_pointillism;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_pointillism(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x & 0xFF) << 16 | (y & 0xFF) << 8;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_pointillism 1080p (dot_size=12)", |b| {
        b.iter(|| {
            apply_pointillism(black_box(&mut fb), black_box(12));
        });
    });
}

criterion_group!(benches, benchmark_pointillism);
criterion_main!(benches);
