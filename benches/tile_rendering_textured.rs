use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{fill_triangle_textured, Texture};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn setup_cube_textured(aspect: f32) -> (Vec<TexturedClipTriangle>, Texture) {
    let mesh = Mesh::cube(2.0);
    // Cube mesh doesn't have UVs by default, generate dummy ones
    let uvs = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];

    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, aspect, 0.1, 100.0);
    let mvp = model * view * projection;

    let transformed_verts: Vec<(Vec3, f32)> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    let mut batch = Vec::new();
    for (i, indices) in mesh.indices.iter().enumerate() {
        let v0 = transformed_verts[indices[0]];
        let v1 = transformed_verts[indices[1]];
        let v2 = transformed_verts[indices[2]];

        // Assign dummy UVs
        let uv0 = uvs[0];
        let uv1 = uvs[1];
        let uv2 = uvs[2];

        batch.push((v0, uv0, v1, uv1, v2, uv2));
    }

    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();
    (batch, texture)
}

fn bench_textured_cube_scanline(c: &mut Criterion) {
    let (batch, texture) = setup_cube_textured(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    c.bench_function("textured_cube_scanline_800x600", |b| {
        b.iter(|| {
            zb.clear();
            for &(v0, uv0, v1, uv1, v2, uv2) in black_box(&batch) {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (v0, uv0),
                    (v1, uv1),
                    (v2, uv2),
                    &texture,
                );
            }
        });
    });
}

fn bench_textured_cube_tiled(c: &mut Criterion) {
    let (batch, texture) = setup_cube_textured(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let mut tr = TileRenderer::new(800, 600);

    c.bench_function("textured_cube_tiled_800x600", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&batch), &texture);
        });
    });
}

criterion_group!(
    benches,
    bench_textured_cube_scanline,
    bench_textured_cube_tiled,
);
criterion_main!(benches);
