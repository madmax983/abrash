use abrash::geometry::{AABB, BoundingSphere};
use abrash::math::{Mat4, Vec3};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_aabb_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("aabb_transform");

    let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let transform = Mat4::translation(10.0, 5.0, -20.0) * Mat4::rotation_y(0.5);

    group.bench_function("transform", |b| {
        b.iter(|| {
            let _ = aabb.transform(&transform);
        })
    });

    group.finish();
}

fn bench_sphere_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("sphere_transform");

    let mut sphere = BoundingSphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 1.0,
    };
    let transform = Mat4::translation(10.0, 5.0, -20.0) * Mat4::rotation_y(0.5);

    group.bench_function("transform", |b| {
        b.iter(|| {
            sphere.transform(&transform);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_aabb_transform, bench_sphere_transform);
criterion_main!(benches);
