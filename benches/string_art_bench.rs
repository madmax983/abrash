use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::math::Vec2;
use abrash_render::experimental::string_art::StringArt;

fn bench_string_art_generate(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_art");

    group.bench_function("generate_100_lines", |b| {
        let width = 100;
        let height = 100;
        let num_pins = 200;
        let num_lines = 100;
        let center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);
        let radius = (width as f32 / 2.0) - 5.0;
        let original_image = vec![0u8; width * height];
        // move the new() outside of iter as precomputation happens there now
        let mut art = StringArt::new(num_pins, center, radius, width, height);

        b.iter(|| {
            let mut image = original_image.clone();
            art.generate(black_box(&mut image), width, height, num_lines);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_string_art_generate);
criterion_main!(benches);
