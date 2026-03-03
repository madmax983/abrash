#![allow(clippy::unreadable_literal)]
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices and MVP for consistent benchmarks
fn setup_cube(aspect: f32) -> (Mesh, Vec<(Vec3, f32)>) {
    let mesh = Mesh::cube(2.0);
    // Rotate and position so it's visible and benefits from texture mapping
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

    (mesh, transformed)
}

fn bench_trilinear_filter(c: &mut Criterion) {
    let (mesh, transformed) = setup_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    // Create a texture and enable Trilinear filtering (this should fail to compile initially)
    let mut texture = Texture::new(64, 64).unwrap();
    texture.filter_mode = FilterMode::Trilinear;

    // Fill with pattern
    for y in 0..64 {
        for x in 0..64 {
            let color = if (x + y) % 2 == 0 {
                0xFFFFFFFF
            } else {
                0xFF000000
            };
            texture.set_pixel(x, y, color);
        }
    }
    texture.generate_mipmaps();

    // Use dummy UVs
    let uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
    ];

    let mut group = c.benchmark_group("mipmapping");

    group.bench_function("trilinear", |b| {
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

    // Also bench Bilinear for comparison
    texture.filter_mode = FilterMode::Bilinear;
    group.bench_function("bilinear", |b| {
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

criterion_group!(benches, bench_trilinear_filter);
criterion_main!(benches);
