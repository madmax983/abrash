use abrash::math::{Mat4, Vec2, Vec3};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vec2_add(c: &mut Criterion) {
    c.bench_function("vec2_add", |b| {
        let a = Vec2::new(1.0, 2.0);
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(a + v));
    });
}

fn bench_vec2_mul(c: &mut Criterion) {
    c.bench_function("vec2_mul", |b| {
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(v * 2.5));
    });
}

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

fn bench_vec3_normalize(c: &mut Criterion) {
    c.bench_function("vec3_normalize", |b| {
        let v = Vec3::new(1.0, 2.0, 3.0);
        b.iter(|| black_box(v.normalize()));
    });
}

criterion_group!(
    benches,
    bench_vec2_add,
    bench_vec2_mul,
    bench_vec3_normalize,
    bench_mat4_mul,
    bench_mat4_transform_point
);
criterion_main!(benches);
