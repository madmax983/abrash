use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_triangle_normal_mapped(c: &mut Criterion) {
    c.bench_function("fill_triangle_normal_mapped", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Texture and Normal Map (simple checkers/flat)
        let diffuse_map = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
        // Normal map: Flat normal (0.5, 0.5, 1.0) stored as color.
        // 0.5 * 255 = 127 = 0x7F. 1.0 * 255 = 255 = 0xFF.
        // ABGR: 0xFF_FF_7F_7F (A=FF, B=FF, G=7F, R=7F).
        // Tangent space normal (0, 0, 1) maps to (0.5, 0.5, 1.0) in 0..1 texture space.
        let normal_map = Texture::checkered(256, 256, 0xFFFF_7F7F, 0xFFFF_7F7F).unwrap();

        // Vertices
        // Position + W
        let p0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let p1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let p2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);

        // UVs
        let uv0 = Vec2::new(0.5, 0.0);
        let uv1 = Vec2::new(0.0, 1.0);
        let uv2 = Vec2::new(1.0, 1.0);

        // Normals (Vertex Normals)
        let n0 = Vec3::new(0.0, 0.0, 1.0);
        let n1 = Vec3::new(0.0, 0.0, 1.0);
        let n2 = Vec3::new(0.0, 0.0, 1.0);

        // Tangents (Tangent + Handedness)
        let t0 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let t1 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let t2 = Vec4::new(1.0, 0.0, 0.0, 1.0);

        // Light
        let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
        let light_color = Vec3::new(1.0, 1.0, 1.0);
        let ambient = Vec3::new(0.1, 0.1, 0.1);

        b.iter(|| {
            zb.clear();
            fill_triangle_normal_mapped(
                &mut fb,
                &mut zb,
                black_box((p0, uv0, n0, t0)),
                black_box((p1, uv1, n1, t1)),
                black_box((p2, uv2, n2, t2)),
                &diffuse_map,
                &normal_map,
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
        });
    });
}

criterion_group!(benches, bench_fill_triangle_normal_mapped);
criterion_main!(benches);
