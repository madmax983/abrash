use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::rasterizer::{fill_triangle_phong, fill_triangle_normal_mapped};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_normal_mapping_vs_phong(c: &mut Criterion) {
    let width = 1024;
    let height = 1024;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Large triangle covering most of the screen
    let p0 = (Vec3::new(0.0, 0.9, 1.0), 1.0);
    let p1 = (Vec3::new(-0.9, -0.9, 1.0), 1.0);
    let p2 = (Vec3::new(0.9, -0.9, 1.0), 1.0);

    let n0 = Vec3::new(0.0, 0.0, 1.0);
    let t0 = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let uv0 = Vec2::new(0.5, 0.0);

    let n1 = Vec3::new(-0.5, 0.0, 0.8);
    let t1 = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let uv1 = Vec2::new(0.0, 1.0);

    let n2 = Vec3::new(0.5, 0.0, 0.8);
    let t2 = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let uv2 = Vec2::new(1.0, 1.0);

    let v0_phong = (p0, n0);
    let v1_phong = (p1, n1);
    let v2_phong = (p2, n2);

    let v0_nm = (p0, uv0, n0, t0);
    let v1_nm = (p1, uv1, n1, t1);
    let v2_nm = (p2, uv2, n2, t2);

    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();
    // Flat normal map (0.5, 0.5, 1.0) -> RGB(128, 128, 255)
    let normal_map = Texture::checkered(256, 256, 0xFF8080FF, 0xFF8080FF).unwrap();

    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let base_color = Vec3::new(1.0, 1.0, 1.0);

    let mut group = c.benchmark_group("shading");

    group.bench_function("phong", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            fill_triangle_phong(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0_phong),
                black_box(v1_phong),
                black_box(v2_phong),
                black_box(base_color),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
        })
    });

    group.bench_function("normal_mapped", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            fill_triangle_normal_mapped(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0_nm),
                black_box(v1_nm),
                black_box(v2_nm),
                black_box(&texture),
                black_box(&normal_map),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
        })
    });

    group.finish();
}

criterion_group!(benches, bench_normal_mapping_vs_phong);
criterion_main!(benches);
