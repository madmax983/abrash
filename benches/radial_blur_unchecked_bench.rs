use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radial_blur::apply_radial_blur;

fn benchmark_radial_blur(c: &mut Criterion) {
    let mut group = c.benchmark_group("radial_blur_opt");

    group.bench_function("swar_16_samples", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        // Pre-fill with a simple pattern
        for y in 0..600 {
            for x in 0..800 {
                fb.set_pixel(x, y, 0xFF0000FF | ((x ^ y) as u32 & 0xFF));
            }
        }
        b.iter(|| {
            apply_radial_blur(&mut fb, 400, 300, 0.5, 16);
        });
    });

    group.bench_function("scalar_fallback_257_samples", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        // Pre-fill with a simple pattern
        for y in 0..600 {
            for x in 0..800 {
                fb.set_pixel(x, y, 0xFF0000FF | ((x ^ y) as u32 & 0xFF));
            }
        }
        b.iter(|| {
            apply_radial_blur(&mut fb, 400, 300, 0.5, 257);
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_radial_blur);
criterion_main!(benches);
