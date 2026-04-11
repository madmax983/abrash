use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
use abrash::experimental::pencil::apply_pencil_sketch;

fn bench_pencil_sketch(c: &mut Criterion) {
    #[cfg(feature = "nova")]
    {
        let mut group = c.benchmark_group("pencil_sketch_filter");

        let width = 1920;
        let height = 1080;
        let mut fb = Framebuffer::new(width, height).unwrap();

        for y in 0..height {
            for x in 0..width {
                let color = 0xFF000000 | ((x % 255) << 16) | ((y % 255) << 8) | ((x + y) % 255);
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        group.bench_function("pencil_sketch_1080p", |b| {
            b.iter(|| {
                apply_pencil_sketch(black_box(&mut fb));
            });
        });

        group.finish();
    }
}

criterion_group!(benches, bench_pencil_sketch);
criterion_main!(benches);
