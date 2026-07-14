#[cfg(feature = "nova")]
use abrash::experimental::jelly::SoftBody;
#[cfg(feature = "nova")]
use abrash::math::Vec3;
#[cfg(feature = "nova")]
use abrash::mesh::Mesh;
use criterion::{Criterion, black_box, criterion_group, criterion_main, BatchSize};

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
fn bench_jelly_creation(c: &mut Criterion) {
    let size = 30; // 30x30 = 900 vertices
    let mesh = create_grid_mesh(size);

    let mut group = c.benchmark_group("jelly_creation");
    group.bench_function("SoftBody::new_30x30", |b| {
        b.iter_batched(
            || mesh.clone(),
            |cloned_mesh| {
                black_box(SoftBody::new(black_box(cloned_mesh), 1.0, 10.0, 0.5).unwrap());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, bench_jelly_creation);

#[cfg(not(feature = "nova"))]
fn bench_jelly_creation(_c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, bench_jelly_creation);

criterion_main!(benches);
