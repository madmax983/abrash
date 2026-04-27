use abrash::framebuffer::Framebuffer;
use abrash::experimental::cross_stitch::{CrossStitchConfig, apply_cross_stitch};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_DD_EE_FF);

    let config = CrossStitchConfig {
        block_size: 10,
        stroke_thickness: 1,
        canvas_color: 0xFF_11_11_11,
    };

    c.bench_function("cross_stitch_800x600", |b| {
        b.iter(|| {
            apply_cross_stitch(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
