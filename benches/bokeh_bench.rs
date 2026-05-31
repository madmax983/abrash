use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::bokeh::{apply_bokeh, BokehConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bokeh_benchmark(c: &mut Criterion) {
    // 256x256 is a reasonable size for an expensive post-process test
    let mut fb = Framebuffer::new(256, 256).unwrap();

    // Draw some bright spots to trigger the bokeh highlight logic
    fb.set_pixel(128, 128, 0xFF_FF_FF_FF);
    fb.set_pixel(64, 64, 0xFF_FF_DD_DD);
    fb.set_pixel(192, 192, 0xFF_DD_FF_DD);

    let config = BokehConfig::default();

    c.bench_function("apply_bokeh_256x256", |b| {
        b.iter(|| apply_bokeh(black_box(&mut fb), black_box(&config)))
    });
}

criterion_group!(benches, bokeh_benchmark);
criterion_main!(benches);
