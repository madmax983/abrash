use abrash_core::math::Vec3;
use abrash_core::mesh::Mesh;
use abrash_render::experimental::voxelizer::{VoxelGrid, Voxelizer};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_voxelizer(c: &mut Criterion) {
    let mut grid = VoxelGrid::new(10, 10, 10, 1.0, Vec3::new(0.0, 0.0, 0.0));
    for z in 0..10 {
        for y in 0..10 {
            for x in 0..10 {
                grid.set(x, y, z, true);
            }
        }
    }

    c.bench_function("voxelizer_to_mesh_10x10x10", |b| {
        b.iter(|| {
            black_box(grid.to_mesh());
        });
    });
}

criterion_group!(benches, bench_voxelizer);
criterion_main!(benches);
