use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Mat4;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::post_process::{SsaoConfig, apply_ssao};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_ssao(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();
    let proj = Mat4::perspective(
        std::f32::consts::PI / 2.0,
        width as f32 / height as f32,
        0.1,
        100.0,
    );
    let config = SsaoConfig {
        radius: 0.5,
        bias: 0.025,
        intensity: 2.0,
    };

    c.bench_function("apply_ssao 1080p", |b| {
        b.iter(|| {
            apply_ssao(
                black_box(&mut fb),
                black_box(&zb),
                black_box(&proj),
                black_box(&config),
            );
        });
    });
}

criterion_group!(benches, bench_ssao);
criterion_main!(benches);
