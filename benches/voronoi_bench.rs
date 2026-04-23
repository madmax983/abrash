use abrash::experimental::voronoi::{VoronoiConfig, apply_voronoi};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn voronoi_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFFFF_FFFF); // White background

    // Add some colors to make use_image_color meaningful
    for y in 0..600 {
        for x in 0..400 {
            fb.set_pixel(x, y, 0xFFFF_0000);
        }
    }

    let config_with_borders = VoronoiConfig {
        num_seeds: 200,
        use_image_color: true,
        metric: 2.0, // Euclidean
        seed: 42,
        border_thickness: 1.0,
        border_color: 0xFF00_0000,
    };

    let config_no_borders = VoronoiConfig {
        num_seeds: 200,
        use_image_color: true,
        metric: 2.0, // Euclidean
        seed: 42,
        border_thickness: 0.0,
        border_color: 0xFF00_0000,
    };

    c.bench_function("voronoi 800x600 200 seeds (with borders)", |b| {
        b.iter(|| {
            apply_voronoi(black_box(&mut fb), black_box(&config_with_borders));
        });
    });

    c.bench_function("voronoi 800x600 200 seeds (no borders)", |b| {
        b.iter(|| {
            apply_voronoi(black_box(&mut fb), black_box(&config_no_borders));
        });
    });
}

criterion_group!(benches, voronoi_benchmark);
criterion_main!(benches);
