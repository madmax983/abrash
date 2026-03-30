use criterion::{black_box, criterion_group, criterion_main, Criterion};

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::dither::{apply_dither, DitherConfig, DitherMode};

fn bench_dither(c: &mut Criterion) {
    let mut group = c.benchmark_group("Dithering Filter");

    // We test at 1080p equivalent
    let width = 1920;
    let height = 1080;

    // Gradient buffer
    let mut fb = Framebuffer::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let val = (x as f32 / width as f32 * 255.0) as u32;
            let color = 0xFF00_0000 | (val << 16) | (val << 8) | val;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config_ordered = DitherConfig {
        mode: DitherMode::Ordered4x4,
        color_depth: 1,
    };

    let config_floyd = DitherConfig {
        mode: DitherMode::FloydSteinberg,
        color_depth: 1,
    };

    group.bench_function("Ordered 4x4 (1-bit)", |b| {
        b.iter_batched(
            || {
                let mut new_fb = Framebuffer::new(width, height).unwrap();
                new_fb.as_mut_slice().copy_from_slice(fb.as_slice());
                new_fb
            },
            |mut cloned_fb| apply_dither(black_box(&mut cloned_fb), black_box(config_ordered)),
            criterion::BatchSize::LargeInput,
        );
    });

    group.bench_function("Floyd-Steinberg (1-bit)", |b| {
        b.iter_batched(
            || {
                let mut new_fb = Framebuffer::new(width, height).unwrap();
                new_fb.as_mut_slice().copy_from_slice(fb.as_slice());
                new_fb
            },
            |mut cloned_fb| apply_dither(black_box(&mut cloned_fb), black_box(config_floyd)),
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_dither);
criterion_main!(benches);
