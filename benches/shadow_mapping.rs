use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::{fill_triangle_phong, fill_triangle_phong_shadowed};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_shadow_mapping(c: &mut Criterion) {
    let width = 1024;
    let height = 1024;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut shadow_map = ZBuffer::new(512, 512).unwrap();
    // Fill shadow map with some data so it's not all INF
    shadow_map.test_and_set(256, 256, 0.5);

    // Large triangle
    let v0 = ((Vec3::new(0.0, 0.9, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v1 = ((Vec3::new(-0.9, -0.9, 1.0), 1.0), Vec3::new(-1.0, 0.0, 0.0));
    let v2 = ((Vec3::new(0.9, -0.9, 1.0), 1.0), Vec3::new(1.0, 0.0, 0.0));

    // World positions (dummy, but distinct to cause interpolation)
    let w0 = Vec3::new(0.0, 10.0, 0.0);
    let w1 = Vec3::new(-10.0, -10.0, 0.0);
    let w2 = Vec3::new(10.0, -10.0, 0.0);

    let v0_s = (v0.0, v0.1, w0);
    let v1_s = (v1.0, v1.1, w1);
    let v2_s = (v2.0, v2.1, w2);

    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let color = Vec3::new(1.0, 1.0, 1.0);
    let light_vp = Mat4::identity();

    let mut group = c.benchmark_group("shadow_mapping");

    group.bench_function("phong_standard", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            fill_triangle_phong(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
        });
    });

    group.bench_function("phong_shadowed_pcf", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            fill_triangle_phong_shadowed(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0_s),
                black_box(v1_s),
                black_box(v2_s),
                black_box(color),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
                black_box(&shadow_map),
                black_box(light_vp),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_shadow_mapping);
criterion_main!(benches);
