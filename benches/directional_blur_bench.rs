use abrash::experimental::directional_blur::apply_directional_blur;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_directional_blur(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 {
                0xFF_FF_FF
            } else {
                0x00_00_00
            };
            fb.set_pixel(x, y, color);
        }
    }

    let mut group = c.benchmark_group("directional_blur_1024x1024");
    group.sample_size(100);

    let config_swar = abrash::experimental::directional_blur::DirectionalBlurConfig {
        dx: 20.0,
        dy: 20.0,
        num_samples: 16,
    };

    group.bench_function("swar_16_samples", |b| {
        b.iter(|| {
            apply_directional_blur(black_box(&mut fb), black_box(&config_swar));
        });
    });

    let config_scalar = abrash::experimental::directional_blur::DirectionalBlurConfig {
        dx: 20.0,
        dy: 20.0,
        num_samples: 257,
    };

    group.bench_function("scalar_fallback_257_samples", |b| {
        b.iter(|| {
            apply_directional_blur(black_box(&mut fb), black_box(&config_scalar));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_directional_blur);
criterion_main!(benches);
