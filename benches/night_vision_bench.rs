use abrash_render::experimental::night_vision::{NightVisionConfig, apply_night_vision};
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

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
