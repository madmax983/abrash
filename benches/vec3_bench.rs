use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vec3_cross(c: &mut Criterion) {
    c.bench_function("vec3_cross", |b| {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(v1.cross(v2)));
    });
}

fn bench_vec3_dot(c: &mut Criterion) {
    c.bench_function("vec3_dot", |b| {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(v1.dot(v2)));
    });
}

criterion_group!(benches, bench_vec3_cross, bench_vec3_dot);
criterion_main!(benches);
