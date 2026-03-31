use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_render::framebuffer::Framebuffer;
use abrash_render::experimental::night_vision::{apply_night_vision, NightVisionConfig};

fn night_vision_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("night_vision");
    group.sample_size(100);

    let width = 1280;
    let height = 720;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF80_8080); // mid-gray

    let config = NightVisionConfig::default();

    group.bench_function("apply_night_vision", |b| {
        b.iter(|| apply_night_vision(black_box(&mut fb), black_box(&config)));
    });

    group.finish();
}

criterion_group!(benches, night_vision_benchmark);
criterion_main!(benches);
