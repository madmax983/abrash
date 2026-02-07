use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::mesh::Mesh;
use abrash::math::{Vec3, Vec2, Mat4};
use abrash::texture::Texture;
use abrash::rasterizer::{fill_triangle_textured, fill_triangle_lit};

fn bench_textured_cube(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mesh = Mesh::cube(2.0);

    // Checkerboard texture
    let tex = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();

    // Fixed MVP matrix
    let model = Mat4::rotation_y(0.78) * Mat4::rotation_x(0.5);
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
    let mvp = model * view * proj;

    c.bench_function("textured_cube_800x600", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();

            for triangle in &mesh.indices {
                let v0 = mesh.vertices[triangle[0]];
                let v1 = mesh.vertices[triangle[1]];
                let v2 = mesh.vertices[triangle[2]];

                let (p0, w0) = mvp.transform_point(v0);
                let (p1, w1) = mvp.transform_point(v1);
                let (p2, w2) = mvp.transform_point(v2);

                // Dummy UVs
                let uv0 = Vec2::new(0.0, 0.0);
                let uv1 = Vec2::new(1.0, 0.0);
                let uv2 = Vec2::new(0.5, 1.0);

                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    black_box(((p0, w0), uv0)),
                    black_box(((p1, w1), uv1)),
                    black_box(((p2, w2), uv2)),
                    &tex
                );
            }
        });
    });
}

fn bench_lit_cube(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mesh = Mesh::cube(2.0);
    // Mesh::cube doesn't set normals, so we compute them or use face normals.
    // For this bench, let's just use a dummy normal or compute it.
    let face_normals = mesh.compute_face_normals();

    // Fixed MVP matrix
    let model = Mat4::rotation_y(0.78) * Mat4::rotation_x(0.5);
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
    let mvp = model * view * proj;

    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let base_color = Vec3::new(0.8, 0.2, 0.2);

    c.bench_function("lit_cube_800x600", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();

            for (i, triangle) in mesh.indices.iter().enumerate() {
                let v0 = mesh.vertices[triangle[0]];
                let v1 = mesh.vertices[triangle[1]];
                let v2 = mesh.vertices[triangle[2]];

                let (p0, w0) = mvp.transform_point(v0);
                let (p1, w1) = mvp.transform_point(v1);
                let (p2, w2) = mvp.transform_point(v2);

                // Transform normal by model matrix (rotation only for correct lighting)
                let normal = model.transform_normal(face_normals[i]);

                fill_triangle_lit(
                    &mut fb,
                    &mut zb,
                    black_box((p0, w0)),
                    black_box((p1, w1)),
                    black_box((p2, w2)),
                    black_box(normal),
                    black_box(base_color),
                    black_box(ambient),
                    black_box(light_dir),
                    black_box(light_color),
                );
            }
        });
    });
}

criterion_group!(benches, bench_textured_cube, bench_lit_cube);
criterion_main!(benches);
