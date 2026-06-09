use abrash::framebuffer::Framebuffer;
use abrash::experimental::pointillism::{apply_pointillism, PointillismConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_pointillism(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with some data
    for y in 0..height {
        for x in 0..width {
            let color = ((x ^ y) as u32) | 0xFF_000000;
            fb.set_pixel(x, y, color);
        }
    }

    let config = PointillismConfig::default();

    c.bench_function("pointillism_800x600", |b| {
        b.iter(|| {
            apply_pointillism(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_pointillism);
criterion_main!(benches);
