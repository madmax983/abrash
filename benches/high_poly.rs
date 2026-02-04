use criterion::{criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::mesh::Mesh;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_gouraud;

fn high_poly_benchmark(c: &mut Criterion) {
    let width = 640;
    let height = 480;

    // Create high poly sphere
    // 50 sectors, 50 stacks -> ~5000 indices
    let sphere = Mesh::sphere(1.0, 50, 50);
    // Pre-compute vertex normals (simple normalized position for sphere centered at origin)
    let normals: Vec<Vec3> = sphere.vertices.iter().map(|v| v.normalize()).collect();

    // Setup View/Projection
    let projection = Mat4::perspective(std::f32::consts::PI / 3.0, width as f32 / height as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let model = Mat4::identity();
    let mvp = projection * (view * model);

    // Lighting (just simple direction)
    let light_dir = Vec3::new(0.5, 0.5, 1.0).normalize();
    let base_color = Vec3::new(1.0, 0.0, 0.0); // Red

    c.bench_function("render_high_poly_sphere", |b| {
        b.iter_with_setup(
            || {
                (Framebuffer::new(width, height).unwrap(), ZBuffer::new(width, height).unwrap())
            },
            |(mut fb, mut zb)| {
                // Render
                for (indices_idx, indices) in sphere.indices.iter().enumerate() {
                     // Note: indices is [usize; 3]
                     let i0 = indices[0];
                     let i1 = indices[1];
                     let i2 = indices[2];

                     let v0_pos = sphere.vertices[i0];
                     let v1_pos = sphere.vertices[i1];
                     let v2_pos = sphere.vertices[i2];

                     let n0 = normals[i0];
                     let n1 = normals[i1];
                     let n2 = normals[i2];

                     // Compute lighting (Gouraud style - per vertex)
                     let l0 = n0.dot(light_dir).max(0.0);
                     let l1 = n1.dot(light_dir).max(0.0);
                     let l2 = n2.dot(light_dir).max(0.0);

                     let c0 = base_color * l0;
                     let c1 = base_color * l1;
                     let c2 = base_color * l2;

                     // Transform
                     let p0 = mvp.transform_point(v0_pos);
                     let p1 = mvp.transform_point(v1_pos);
                     let p2 = mvp.transform_point(v2_pos);

                     fill_triangle_gouraud(&mut fb, &mut zb, (p0, c0), (p1, c1), (p2, c2));
                }
            }
        )
    });
}

criterion_group!(benches, high_poly_benchmark);
criterion_main!(benches);
