use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices, UVs, and MVP for consistent benchmarks
fn setup_textured_cube(aspect: f32) -> (Mesh, Vec<(Vec3, f32)>, Vec<Vec2>) {
    let mesh = Mesh::cube(2.0);
    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, aspect, 0.1, 100.0);
    let mvp = model * view * projection;

    let transformed: Vec<(Vec3, f32)> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    // Generate dummy UVs for each vertex
    let uvs: Vec<Vec2> = mesh.vertices.iter().enumerate().map(|(i, _)| {
        Vec2::new((i % 2) as f32, ((i / 2) % 2) as f32)
    }).collect();

    (mesh, transformed, uvs)
}

fn bench_cube_textured_tiled(c: &mut Criterion) {
    let (mesh, transformed, uvs) = setup_textured_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let mut tr = TileRenderer::new(800, 600);
    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();

    let batch: Vec<TexturedClipTriangle> = mesh
        .indices
        .iter()
        .map(|idx| {
            (
                transformed[idx[0]], uvs[idx[0]],
                transformed[idx[1]], uvs[idx[1]],
                transformed[idx[2]], uvs[idx[2]],
            )
        })
        .collect();

    c.bench_function("fill_cube_textured_tiled", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, &texture, black_box(&batch));
        });
    });
}

fn bench_cube_textured_scanline(c: &mut Criterion) {
    let (mesh, transformed, uvs) = setup_textured_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();

    c.bench_function("fill_cube_textured_scanline", |b| {
        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (transformed[indices[0]], uvs[indices[0]]),
                    (transformed[indices[1]], uvs[indices[1]]),
                    (transformed[indices[2]], uvs[indices[2]]),
                    &texture,
                );
            }
        });
    });
}

criterion_group!(
    benches,
    bench_cube_textured_tiled,
    bench_cube_textured_scanline,
);
criterion_main!(benches);
