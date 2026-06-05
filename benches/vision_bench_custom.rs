use abrash_render::experimental::vision::{VisionConfig, VisionMode, apply_vision};
use abrash_render::framebuffer::Framebuffer;
use abrash_render::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn vision_custom_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("vision_custom");
    group.sample_size(100);

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();

    fb.clear(0xFF80_8080); // mid-gray

    let config = VisionConfig {
        mode: VisionMode::Night,
        time: 0.0,
        intensity: 1.0,
    };

    group.bench_function("apply_vision_night_mode_800x600", |b| {
        b.iter(|| {
            apply_vision(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });

    group.finish();
}

criterion_group!(benches, vision_custom_benchmark);
criterion_main!(benches);
