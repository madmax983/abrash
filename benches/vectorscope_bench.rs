use abrash::experimental::vectorscope::{VectorscopeConfig, apply_vectorscope};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vectorscope(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF55_5555); // Some base color to analyze

    let config = VectorscopeConfig {
        center_x: 150,
        center_y: 150,
        radius: 120,
        intensity: 0.05,
        draw_background: true,
        draw_grid: true,
    };

    c.bench_function("apply_vectorscope_800x600", |b| {
        b.iter(|| {
            apply_vectorscope(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_vectorscope);
criterion_main!(benches);
