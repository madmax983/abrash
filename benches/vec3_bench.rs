use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vec3_ops(c: &mut Criterion) {
    let v1 = Vec3::new(1.0, 2.0, 3.0);
    let v2 = Vec3::new(4.0, 5.0, 6.0);

    c.bench_function("vec3_dot", |b| b.iter(|| black_box(v1).dot(black_box(v2))));

    c.bench_function("vec3_cross", |b| {
        b.iter(|| black_box(v1).cross(black_box(v2)));
    });

    c.bench_function("vec3_normalize", |b| b.iter(|| black_box(v1).normalize()));

    c.bench_function("vec3_fast_normalize", |b| {
        b.iter(|| black_box(v1).fast_normalize());
    });
}

criterion_group!(benches, bench_vec3_ops);
criterion_main!(benches);
