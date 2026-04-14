use abrash::experimental::dither::{apply_dither, DitherConfig, DitherMode};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_dither(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern
    for y in 0..height {
        for x in 0..width {
            let val = (x as f32 / width as f32 * 255.0) as u32;
            let color = 0xFF00_0000 | (val << 16) | (val << 8) | val;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let mut group = c.benchmark_group("dither");

    let config_ordered = DitherConfig {
        mode: DitherMode::Ordered4x4,
        color_depth: 1,
    };

    group.bench_function("apply_dither_ordered_4x4_1080p", |b| {
        b.iter(|| {
            apply_dither(black_box(&mut fb), black_box(config_ordered));
        });
    });

    let config_fs = DitherConfig {
        mode: DitherMode::FloydSteinberg,
        color_depth: 1,
    };

    group.bench_function("apply_dither_floyd_steinberg_1080p", |b| {
        b.iter(|| {
            apply_dither(black_box(&mut fb), black_box(config_fs));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_dither);
criterion_main!(benches);
