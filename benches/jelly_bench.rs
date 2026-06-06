#[cfg(feature = "nova")]
use abrash::experimental::jelly::SoftBody;
#[cfg(feature = "nova")]
use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene};
#[cfg(feature = "nova")]
use abrash::math::Vec3;
#[cfg(feature = "nova")]
use abrash::mesh::Mesh;
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn create_grid_mesh(size: usize) -> Mesh {
    let mut mesh = Mesh::new();
    for y in 0..size {
        for x in 0..size {
            mesh.vertices.push(Vec3::new(x as f32, 0.0, y as f32));
        }
    }
    for y in 0..(size - 1) {
        for x in 0..(size - 1) {
            let i0 = y * size + x;
            let i1 = y * size + x + 1;
            let i2 = (y + 1) * size + x;
            let i3 = (y + 1) * size + x + 1;
            mesh.indices.push([i0, i1, i2]);
            mesh.indices.push([i1, i3, i2]);
        }
    }
    mesh
}

#[cfg(feature = "nova")]
fn bench_jelly_collide_sdf(c: &mut Criterion) {
    let size = 30; // 30x30 = 900 vertices
    let mesh = create_grid_mesh(size);
    let mut softbody = SoftBody::new(mesh, 1.0, 10.0, 0.5).expect("Failed to create SoftBody");

    // Create a sphere scene right in the middle
    let mut scene = SdfScene::new();
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere {
            center: Vec3::new(15.0, 0.0, 15.0),
            radius: 5.0,
        },
        color: 0,
    });

    let mut group = c.benchmark_group("jelly");
    group.bench_function("update_30x30", |b| {
        b.iter(|| {
            softbody.update(0.016);
        });
    });
    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, bench_jelly_collide_sdf);

#[cfg(not(feature = "nova"))]
fn bench_jelly_collide_sdf(c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, bench_jelly_collide_sdf);

criterion_main!(benches);
