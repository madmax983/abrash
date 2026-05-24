use abrash_render::experimental::vision::{VisionConfig, VisionMode, apply_vision};
use abrash_render::framebuffer::Framebuffer;
use abrash_render::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn vision_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("vision_post_process");
    group.sample_size(100);

    let width = 1280;
    let height = 720;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();

    fb.clear(0xFF80_8080); // mid-gray

    let config_night = VisionConfig {
        mode: VisionMode::Night,
        time: 0.0,
        intensity: 1.0,
    };

    group.bench_function("apply_vision_night_mode_720p", |b| {
        b.iter(|| {
            apply_vision(black_box(&mut fb), black_box(&zb), black_box(&config_night));
        });
    });

    let config_thermal = VisionConfig {
        mode: VisionMode::Thermal,
        time: 0.0,
        intensity: 1.0,
    };

    group.bench_function("apply_vision_thermal_mode_720p", |b| {
        b.iter(|| {
            apply_vision(black_box(&mut fb), black_box(&zb), black_box(&config_thermal));
        });
    });

    group.finish();
}

criterion_group!(benches, vision_benchmark);
criterion_main!(benches);
