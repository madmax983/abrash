use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::mesh::Mesh;

fn bench_mesh_plane(c: &mut Criterion) {
    c.bench_function("mesh_plane_100x100", |b| {
        b.iter(|| {
            black_box(Mesh::plane(10.0, 100));
        });
    });
}

fn bench_mesh_sphere(c: &mut Criterion) {
    c.bench_function("mesh_sphere_64x64", |b| {
        b.iter(|| {
            black_box(Mesh::sphere(5.0, 64, 64));
        });
    });
}

fn bench_mesh_cylinder(c: &mut Criterion) {
    c.bench_function("mesh_cylinder_64x64", |b| {
        b.iter(|| {
            black_box(Mesh::cylinder(2.0, 10.0, 64, 64));
        });
    });
}

fn bench_mesh_torus(c: &mut Criterion) {
    c.bench_function("mesh_torus_64x32", |b| {
        b.iter(|| {
            black_box(Mesh::torus(3.0, 1.0, 64, 32));
        });
    });
}

criterion_group!(
    benches,
    bench_mesh_plane,
    bench_mesh_sphere,
    bench_mesh_cylinder,
    bench_mesh_torus
);
criterion_main!(benches);
