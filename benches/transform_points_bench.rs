use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::transform::Transform;
use abrash_core::math::Vec3;
use abrash_core::quat::Quat;

fn transform_points_optimized(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(3.0, -2.0, 5.0),
        Quat::from_euler(0.4, -0.2, 0.1),
        Vec3::new(2.0, 3.0, 0.5),
    );
    let points = vec![Vec3::new(1.0, 2.0, 3.0); 1000];
    let mut out = Vec::new();

    c.bench_function("transform_points_opt_1000", |b| {
        b.iter(|| {
            transform.transform_points_into(std::hint::black_box(&points), &mut out);
            std::hint::black_box(&out);
        });
    });
}

fn transform_vectors_optimized(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(3.0, -2.0, 5.0),
        Quat::from_euler(0.4, -0.2, 0.1),
        Vec3::new(2.0, 3.0, 0.5),
    );
    let vectors = vec![Vec3::new(1.0, 2.0, 3.0); 1000];
    let mut out = Vec::new();

    c.bench_function("transform_vectors_opt_1000", |b| {
        b.iter(|| {
            transform.transform_vectors_into(std::hint::black_box(&vectors), &mut out);
            std::hint::black_box(&out);
        });
    });
}

criterion_group!(benches, transform_points_optimized, transform_vectors_optimized);
criterion_main!(benches);
