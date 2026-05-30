use abrash_render::experimental::string_art::StringArt;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn string_art_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let center = Vec2::new(400.0, 300.0);

    // Benchmark 1: Low point count
    let art_low = StringArt::new(100, 2.5, 250.0, center);
    c.bench_function("string_art_100_points", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            art_low.render(black_box(&mut fb));
        })
    });

    // Benchmark 2: High point count
    let art_high = StringArt::new(1000, 21.0, 250.0, center);
    c.bench_function("string_art_1000_points", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            art_high.render(black_box(&mut fb));
        })
    });
}

criterion_group!(benches, string_art_benchmark);
criterion_main!(benches);
