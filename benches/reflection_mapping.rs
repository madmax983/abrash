use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_reflection;
use abrash::skybox::Cubemap;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_triangle_reflection(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a dummy cubemap
    let mut tex = Texture::new(64, 64).unwrap();
    tex.pixels.fill(0xFFFFFFFF);
    let faces = [
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
    ];
    let cubemap = Cubemap::new(faces);

    // Vertices ((ClipPos, W), Normal, WorldPos)
    let v0 = (
        (Vec3::new(0.0, 0.9, 0.5), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 2.0, 0.0),
    );
    let v1 = (
        (Vec3::new(-0.9, -0.9, 0.5), 1.0),
        Vec3::new(-0.7, -0.7, 0.7),
        Vec3::new(-2.0, -2.0, 0.0),
    );
    let v2 = (
        (Vec3::new(0.9, -0.9, 0.5), 1.0),
        Vec3::new(0.7, -0.7, 0.7),
        Vec3::new(2.0, -2.0, 0.0),
    );

    let camera_pos = Vec3::new(0.0, 0.0, 5.0);

    c.bench_function("fill_triangle_reflection", |b| {
        b.iter(|| {
            zb.clear();
            fill_triangle_reflection(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(camera_pos),
                black_box(&cubemap),
            );
        });
    });
}

criterion_group!(benches, bench_fill_triangle_reflection);
criterion_main!(benches);
