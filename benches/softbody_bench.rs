use abrash::experimental::jelly::SoftBody;
use abrash::mesh::Mesh;
use abrash::math::Vec3;
use criterion::{criterion_group, criterion_main, Criterion};

fn create_grid_mesh(size: usize) -> Mesh {
    let mut mesh = Mesh::new();
    // Create vertices
    for y in 0..size {
        for x in 0..size {
            mesh.vertices.push(Vec3::new(x as f32, 0.0, y as f32));
        }
    }
    // Create indices (two triangles per quad)
    for y in 0..(size - 1) {
        for x in 0..(size - 1) {
            let i0 = y * size + x;
            let i1 = y * size + x + 1;
            let i2 = (y + 1) * size + x;
            let i3 = (y + 1) * size + x + 1;
            // First triangle (i0, i1, i2)
            mesh.indices.push([i0, i1, i2]);
            // Second triangle (i1, i3, i2)
            mesh.indices.push([i1, i3, i2]);
        }
    }
    mesh
}

fn bench_softbody_update(c: &mut Criterion) {
    // 20x20 grid = 400 vertices
    let size = 20;
    let mesh = create_grid_mesh(size);
    let mut softbody = SoftBody::new(mesh, 1.0, 10.0, 0.5).expect("Failed to create SoftBody");

    let mut group = c.benchmark_group("softbody");
    group.bench_function("update_20x20", |b| {
        b.iter(|| {
            // Update step (0.016s = ~60fps)
            softbody.update(0.016);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_softbody_update);
criterion_main!(benches);
