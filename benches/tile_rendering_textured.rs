use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{fill_triangle_textured, Texture};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices and MVP for consistent benchmarks
fn setup_cube(aspect: f32) -> (Mesh, Vec<(Vec3, f32, Vec2)>) {
    let mesh = Mesh::cube(2.0);
    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, aspect, 0.1, 100.0);
    let mvp = model * view * projection;

    // Fake UVs
    let transformed: Vec<(Vec3, f32, Vec2)> = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let (p, w) = mvp.transform_point(*v);
            let u = if i % 2 == 0 { 0.0 } else { 1.0 };
            let v = if (i / 2) % 2 == 0 { 0.0 } else { 1.0 };
            (p, w, Vec2::new(u, v))
        })
        .collect();

    (mesh, transformed)
}

fn bench_cube_textured_tiled(c: &mut Criterion) {
    let (mesh, transformed) = setup_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let mut tr = TileRenderer::new(800, 600);
    let texture = Texture::new(256, 256).unwrap();

    let batch: Vec<TexturedClipTriangle> = mesh
        .indices
        .iter()
        .map(|idx| {
            let v0 = transformed[idx[0]];
            let v1 = transformed[idx[1]];
            let v2 = transformed[idx[2]];
            (
                (v0.0, v0.1), v0.2,
                (v1.0, v1.1), v1.2,
                (v2.0, v2.1), v2.2,
            )
        })
        .collect();

    c.bench_function("fill_cube_textured_tiled", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&batch), &texture);
        });
    });
}

fn bench_cube_textured_scanline(c: &mut Criterion) {
    let (mesh, transformed) = setup_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let texture = Texture::new(256, 256).unwrap();

    c.bench_function("fill_cube_textured_scanline", |b| {
        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                let v0 = transformed[indices[0]];
                let v1 = transformed[indices[1]];
                let v2 = transformed[indices[2]];
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    black_box(((v0.0, v0.1), v0.2)),
                    black_box(((v1.0, v1.1), v1.2)),
                    black_box(((v2.0, v2.1), v2.2)),
                    black_box(&texture),
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
