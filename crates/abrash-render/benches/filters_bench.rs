use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_sobel;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn bench_sobel(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sobel Filter");

    let resolutions = [
        (320, 240, "320x240"),
        (800, 600, "800x600"),
        (1920, 1080, "1920x1080"),
    ];

    for (width, height, label) in resolutions {
        let mut fb = Framebuffer::new(width, height).unwrap();
        // fill with noise
        for y in 0..height {
            for x in 0..width {
                fb.set_pixel(
                    x as i32,
                    y as i32,
                    0xFF00_0000 | (x as u32 % 256) << 16 | (y as u32 % 256) << 8,
                );
            }
        }

        group.bench_with_input(
            BenchmarkId::new("scalar", label),
            &(width, height),
            |b, _| {
                b.iter(|| {
                    apply_sobel(&mut fb);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_sobel);
criterion_main!(benches);
