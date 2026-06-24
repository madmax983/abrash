use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_point_lit;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_new_fill_triangle_point_lit(c: &mut Criterion) {
    let mut group = c.benchmark_group("New Point Light Rasterization");

    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let w0 = Vec3::new(0.0, 9.0, 0.0);
    let w1 = Vec3::new(-9.0, -9.0, 0.0);
    let w2 = Vec3::new(9.0, -9.0, 0.0);

    let v0 = (
        (Vec3::new(0.0, 0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w0,
    );
    let v1 = (
        (Vec3::new(-0.9, -0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w1,
    );
    let v2 = (
        (Vec3::new(0.9, -0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w2,
    );

    let light_pos = Vec3::new(0.0, 0.0, 5.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let attenuation = Vec3::new(0.0, 0.1, 0.01);
    let color = Vec3::new(1.0, 1.0, 1.0);

    group.bench_function("Optimized SIMD Point Lit Refined", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            fill_triangle_point_lit(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
                black_box(light_pos),
                black_box(light_color),
                black_box(attenuation),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_new_fill_triangle_point_lit);
criterion_main!(benches);
