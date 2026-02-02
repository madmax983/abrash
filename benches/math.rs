use abrash::math::{Mat4, Vec3};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_mat4_mul(c: &mut Criterion) {
    c.bench_function("mat4_mul", |b| {
        let a = Mat4::rotation_y(0.5);
        let m = Mat4::translation(1.0, 2.0, 3.0);
        b.iter(|| black_box(a * m));
    });
}

fn bench_mat4_transform_point(c: &mut Criterion) {
    c.bench_function("mat4_transform_point", |b| {
        let m = Mat4::rotation_y(0.5);
        let v = Vec3::new(1.0, 2.0, 3.0);
        b.iter(|| black_box(m.transform_point(v)));
    });
}

criterion_group!(
    benches,
    bench_mat4_mul,
    bench_mat4_transform_point
);
criterion_main!(benches);
