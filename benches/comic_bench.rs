use abrash::experimental::comic::{apply_comic, ComicConfig};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_apply_comic(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient to process
    for y in 0..height {
        for x in 0..width {
            let r = ((x as f32 / width as f32) * 255.0) as u32;
            let g = ((y as f32 / height as f32) * 255.0) as u32;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | 128;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Add some "edges" to detect
    for y in 100..200 {
        for x in 100..200 {
            fb.set_pixel(x as i32, y as i32, 0xFF_FFFFFF);
        }
    }

    let config_no_halftone = ComicConfig {
        paint_radius: 2,
        edge_threshold: 50,
        use_halftone: false,
        ..Default::default()
    };

    let config_halftone = ComicConfig {
        paint_radius: 2,
        edge_threshold: 50,
        use_halftone: true,
        halftone_dot_size: 3.0,
        halftone_angle: std::f32::consts::PI / 4.0,
    };

    let mut group = c.benchmark_group("comic_filter");

    group.bench_function("apply_comic_no_halftone", |b| {
        b.iter(|| {
            apply_comic(&mut fb, &config_no_halftone);
        })
    });

    group.bench_function("apply_comic_with_halftone", |b| {
        b.iter(|| {
            apply_comic(&mut fb, &config_halftone);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_apply_comic);
criterion_main!(benches);
