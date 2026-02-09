use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_clipping_grid(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Generate a 50x50 grid of quads (5000 triangles)
    // Centered at 0,0, extending from -25 to +25 in X and Y
    let grid_size = 50;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for y in 0..=grid_size {
        for x in 0..=grid_size {
            vertices.push(Vec3::new(x as f32 - 25.0, y as f32 - 25.0, 0.0));
        }
    }

    for y in 0..grid_size {
        for x in 0..grid_size {
            let i0 = y * (grid_size + 1) + x;
            let i1 = i0 + 1;
            let i2 = (y + 1) * (grid_size + 1) + x;
            let i3 = i2 + 1;

            // Quad -> 2 triangles
            indices.push([i0, i1, i2]);
            indices.push([i1, i3, i2]);
        }
    }

    // Setup transformation
    // Camera at (0, 0, -10), looking at (0, 0, 0).
    // The grid is at z=0.
    // Move grid to the right by 20 units. Center is at (20, 0, 0).
    // View frustum at z=0 (distance 10) with 90 deg FOV covers approx -10 to +10 in X.
    // Grid extends from -5 to +45 in X (centered at 20, width 50).
    // Visible range X: -10 to 10.
    // Grid range X: -5 to 45.
    // Overlap: -5 to 10. (15 units visible).
    // Off-screen: 10 to 45 (35 units invisible).
    // So ~70% of the grid is off the right edge and should be clipped.

    let model = Mat4::translation(20.0, 0.0, 0.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -10.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
    // Matrix multiplication order for row vectors: v * M1 * M2 * M3
    // v_world = v_local * Model
    // v_view = v_world * View
    // v_clip = v_view * Projection
    // So MVP = Model * View * Projection
    let mvp = model * view * projection;

    // Pre-transform vertices to simulate the pipeline state just before rasterization/clipping
    let transformed_verts: Vec<(Vec3, f32)> = vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    c.bench_function("fill_grid_clipping", |b| {
        b.iter(|| {
            zb.clear();

            for tri in &indices {
                let v0 = transformed_verts[tri[0]];
                let v1 = transformed_verts[tri[1]];
                let v2 = transformed_verts[tri[2]];

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

criterion_group!(benches, bench_clipping_grid);
criterion_main!(benches);
