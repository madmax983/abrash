use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_culling_cube(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    let mesh = Mesh::cube(2.0);

    // Setup transformation
    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5); // Rotate to show 3 faces
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0);
    let mvp = model * view * projection;

    // Pre-transform vertices to simulate the pipeline state just before rasterization
    let transformed_verts: Vec<(Vec3, f32)> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    c.bench_function("fill_cube_culling", |b| {
        b.iter(|| {
            zb.clear();

            for indices in &mesh.indices {
                let v0 = transformed_verts[indices[0]];
                let v1 = transformed_verts[indices[1]];
                let v2 = transformed_verts[indices[2]];

                fill_triangle_3d(
                    &mut fb,
                    &mut zb,
                    black_box(v0),
                    black_box(v1),
                    black_box(v2),
                    black_box(0xFFFF_FFFF),
                );
            }
        });
    });
}

criterion_group!(benches, bench_culling_cube);
criterion_main!(benches);
