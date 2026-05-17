use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Mat4;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::post_process::ssao::{SsaoConfig, apply_ssao};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn ssao_benchmark(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill ZBuffer with something non-empty
    for y in 0..height {
        for x in 0..width {
            let depth = 0.5 + (x as f32 / width as f32) * 0.4;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    let config = SsaoConfig::default();
    let proj = Mat4::identity();

    c.bench_function("apply_ssao", |b| {
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

criterion_group!(benches, ssao_benchmark);
criterion_main!(benches);
