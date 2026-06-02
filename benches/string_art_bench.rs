use abrash_render::experimental::string_art::StringArt;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn string_art_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("String Art Generation");

    // Standard high-resolution parameters
    let num_pegs = 200;
    let width = 500;
    let height = 500;
    let num_lines = 1000;

    let art = StringArt::new(num_pegs, width, height);
    let mut darkness_map = vec![0u8; width * height];

    // Create a mock image (a simple gradient or pattern)
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - (width as f32 / 2.0);
            let dy = y as f32 - (height as f32 / 2.0);
            let dist = (dx * dx + dy * dy).sqrt();
            let max_dist = (width.min(height) as f32) / 2.0;
            let intensity = (1.0 - (dist / max_dist).clamp(0.0, 1.0)) * 255.0;
            darkness_map[y * width + x] = intensity as u8;
        }
    }

    group.bench_function("generate_1000_lines", |b| {
        b.iter(|| {
            let sequence = art.generate(black_box(&darkness_map), black_box(num_lines));
            black_box(sequence);
        });
    });

    group.finish();
}

criterion_group!(benches, string_art_benchmark);
criterion_main!(benches);
