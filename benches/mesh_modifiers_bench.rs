use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash::experimental::modifiers::{displace_noise, taper, twist};
use abrash::experimental::procedural_mesh::TerrainGenerator;
use abrash::experimental::isosurface::extract_isosurface;
use abrash::math::Vec3;

fn bench_modifiers(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_modifiers");

    // Generate a reasonably large mesh (100x100 grid = 10,201 vertices)
    let original_mesh = TerrainGenerator::generate_plane(100.0, 100.0, 100);

    group.bench_function("twist", |b| {
        b.iter_batched(
            || original_mesh.clone(),
            |mut mesh| twist(black_box(&mut mesh), 3.14),
            criterion::BatchSize::LargeInput,
        );
    });

    group.bench_function("taper", |b| {
        b.iter_batched(
            || original_mesh.clone(),
            |mut mesh| taper(black_box(&mut mesh), 0.5),
            criterion::BatchSize::LargeInput,
        );
    });

    group.bench_function("displace_noise", |b| {
        b.iter_batched(
            || original_mesh.clone(),
            |mut mesh| displace_noise(black_box(&mut mesh), 2.0, 42),
            criterion::BatchSize::LargeInput,
        );
    });

    let sphere_sdf = |p: Vec3| p.length() - 1.0;
    let min = Vec3::new(-1.5, -1.5, -1.5);
    let max = Vec3::new(1.5, 1.5, 1.5);

    group.bench_function("extract_isosurface_res10", |b| {
        b.iter(|| {
            extract_isosurface(black_box(sphere_sdf), black_box(min), black_box(max), black_box(10))
        });
    });

    group.bench_function("extract_isosurface_res20", |b| {
        b.iter(|| {
            extract_isosurface(black_box(sphere_sdf), black_box(min), black_box(max), black_box(20))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_modifiers);
criterion_main!(benches);
