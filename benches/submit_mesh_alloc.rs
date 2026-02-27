use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn setup_heavy_mesh() -> (Mesh, Vec<(Vec3, f32)>) {
    // 50x50x50 cube grid = 125,000 cubes. Too many?
    // Let's do 20x20x20 = 8000 cubes * 12 triangles = 96,000 triangles.
    // This is enough to stress allocation.
    let count = 20;
    let mut vertices = Vec::with_capacity(count * count * count * 8);
    let mut indices = Vec::with_capacity(count * count * count * 12); // indices are [usize; 3]

    for z in 0..count {
        for y in 0..count {
            for x in 0..count {
                let base = vertices.len();
                let xf = x as f32;
                let yf = y as f32;
                let zf = z as f32;

                vertices.push(Vec3::new(xf, yf, zf));
                vertices.push(Vec3::new(xf + 1.0, yf, zf));
                vertices.push(Vec3::new(xf + 1.0, yf + 1.0, zf));
                vertices.push(Vec3::new(xf, yf + 1.0, zf));
                vertices.push(Vec3::new(xf, yf, zf + 1.0));
                vertices.push(Vec3::new(xf + 1.0, yf, zf + 1.0));
                vertices.push(Vec3::new(xf + 1.0, yf + 1.0, zf + 1.0));
                vertices.push(Vec3::new(xf, yf + 1.0, zf + 1.0));

                // Just adding a few triangles per cube to simulate load
                indices.push([base, base + 1, base + 2]);
                indices.push([base, base + 2, base + 3]);
                indices.push([base + 4, base + 5, base + 6]);
                indices.push([base + 4, base + 6, base + 7]);
            }
        }
    }

    let mesh = Mesh {
        vertices,
        indices,
        uvs: vec![],
        normals: vec![],
        tangents: vec![],
    };

    let mvp = Mat4::perspective(1.57, 1.33, 0.1, 1000.0)
        * Mat4::translation(0.0, 0.0, -50.0);

    let transformed: Vec<(Vec3, f32)> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    (mesh, transformed)
}

fn bench_submit_mesh(c: &mut Criterion) {
    let (mesh, transformed) = setup_heavy_mesh();
    let mut tr = TileRenderer::new(1920, 1080);

    c.bench_function("submit_mesh_parallel", |b| {
        b.iter(|| {
            tr.begin_frame(); // Clears internal buffers
            tr.submit_mesh(
                black_box(&mesh.indices),
                black_box(&transformed),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

criterion_group!(benches, bench_submit_mesh);
criterion_main!(benches);
