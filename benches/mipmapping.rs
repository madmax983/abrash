use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices and MVP
fn setup_cube(aspect: f32, z_dist: f32) -> (Mesh, Vec<(Vec3, f32)>) {
    let mesh = Mesh::cube(2.0);
    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -z_dist),
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

    (mesh, transformed)
}

fn bench_mipmapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("mipmapping");

    // Create a large texture to emphasize cache effects
    let mut texture = Texture::new(1024, 1024).unwrap();
    // Fill with checker pattern
    let block_w = 64;
    let block_h = 64;
    for y in 0..1024 {
        for x in 0..1024 {
            let check = ((x / block_w) + (y / block_h)) & 1 == 0;
            texture.set_pixel(x, y, if check { 0xFFFFFFFF } else { 0xFF000000 });
        }
    }
    texture.generate_mipmaps();

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Case 1: Close up object (LOD 0)
    // Trilinear should be slower due to overhead
    group.bench_function("close_bilinear", |b| {
        let (mesh, transformed) = setup_cube(width as f32 / height as f32, 5.0);
        texture.filter_mode = FilterMode::Bilinear;

        // Dummy UVs
        let uvs = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];

        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (black_box(transformed[indices[0]]), uvs[0]),
                    (black_box(transformed[indices[1]]), uvs[1]),
                    (black_box(transformed[indices[2]]), uvs[2]),
                    &texture,
                );
            }
        });
    });

    group.bench_function("close_trilinear", |b| {
        let (mesh, transformed) = setup_cube(width as f32 / height as f32, 5.0);
        texture.filter_mode = FilterMode::Trilinear;

        let uvs = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];

        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (black_box(transformed[indices[0]]), uvs[0]),
                    (black_box(transformed[indices[1]]), uvs[1]),
                    (black_box(transformed[indices[2]]), uvs[2]),
                    &texture,
                );
            }
        });
    });

    // Case 2: Far away object (High LOD)
    // Trilinear might still be slower due to blending, but cache should be better.
    // If we compare to Bilinear (which samples level 0), Trilinear sampling level N should be much faster due to cache.
    // Wait, my Bilinear implementation currently always samples level 0.
    // So Trilinear should win on cache hits significantly if texture is large (1024x1024 = 4MB) and we render small object.

    group.bench_function("far_bilinear", |b| {
        // Move cube very far
        let (mesh, transformed) = setup_cube(width as f32 / height as f32, 50.0);
        texture.filter_mode = FilterMode::Bilinear;
        let uvs = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];

        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (black_box(transformed[indices[0]]), uvs[0]),
                    (black_box(transformed[indices[1]]), uvs[1]),
                    (black_box(transformed[indices[2]]), uvs[2]),
                    &texture,
                );
            }
        });
    });

    group.bench_function("far_trilinear", |b| {
        let (mesh, transformed) = setup_cube(width as f32 / height as f32, 50.0);
        texture.filter_mode = FilterMode::Trilinear;
        let uvs = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];

        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (black_box(transformed[indices[0]]), uvs[0]),
                    (black_box(transformed[indices[1]]), uvs[1]),
                    (black_box(transformed[indices[2]]), uvs[2]),
                    &texture,
                );
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_mipmapping);
criterion_main!(benches);
