use abrash::experimental::directional_blur::apply_directional_blur;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_directional_blur(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFF } else { 0x000000 };
            fb.set_pixel(x, y, color);
        }
    }

    let config = abrash::experimental::directional_blur::DirectionalBlurConfig {
        dx: 20.0,
        dy: 10.0,
        num_samples: 16,
    };
    c.bench_function("directional_blur_1024x1024", |b| {
        b.iter(|| {
            apply_directional_blur(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_directional_blur);
criterion_main!(benches);
