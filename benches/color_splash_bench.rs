use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::color_splash::{apply_color_splash, ColorSplashConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_color_splash(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = ColorSplashConfig::default();

    c.bench_function("color_splash_1080p", |b| {
        b.iter(|| apply_color_splash(black_box(&mut fb), black_box(&config)))
    });
}

criterion_group!(benches, benchmark_color_splash);
criterion_main!(benches);
