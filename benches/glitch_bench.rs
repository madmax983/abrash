use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::glitch::{GlitchParams, apply_glitch};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_glitch(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient so there's data to process
    for y in 0..height {
        for x in 0..width {
            let color = (x % 255) | ((y % 255) << 8) | 0xFF000000;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let mut group = c.benchmark_group("Glitch Filter");

    let mut params = GlitchParams::default();
    params.intensity = 1.0;

    group.bench_function("800x600", |b| {
        b.iter(|| {
            apply_glitch(&mut fb, &params);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_glitch);
criterion_main!(benches);
