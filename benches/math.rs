use abrash::math::{Mat2, Mat4, Vec2, Vec3};
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

fn bench_mat2_transform(c: &mut Criterion) {
    c.bench_function("mat2_transform", |b| {
        let mat = Mat2::rotation(0.785); // 45 degrees
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(mat.transform(v)));
    });
}

fn bench_mat2_batch_transform(c: &mut Criterion) {
    c.bench_function("mat2_batch_transform_100", |b| {
        let mat = Mat2::rotation(0.785);
        let vertices: Vec<Vec2> = (0..100).map(|i| Vec2::new(i as f32, i as f32)).collect();

        b.iter(|| {
            vertices
                .iter()
                .map(|&v| mat.transform(v))
                .collect::<Vec<_>>()
        });
    });
}

fn bench_mat2_transform_in_place(c: &mut Criterion) {
    c.bench_function("mat2_transform_in_place_100", |b| {
        let mat = Mat2::rotation(0.785);
        let mut vertices: Vec<Vec2> = (0..100).map(|i| Vec2::new(i as f32, i as f32)).collect();

        b.iter(|| {
            mat.transform_in_place(black_box(&mut vertices));
        });
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

fn bench_vec3_reflect(c: &mut Criterion) {
    c.bench_function("vec3_reflect", |b| {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let n = Vec3::new(0.0, 1.0, 0.0);
        b.iter(|| black_box(v.reflect(n)));
    });
}

fn bench_mat4_orthographic(c: &mut Criterion) {
    c.bench_function("mat4_orthographic", |b| {
        b.iter(|| black_box(Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 0.1, 100.0)));
    });
}

fn bench_mat4_inverse(c: &mut Criterion) {
    c.bench_function("mat4_inverse", |b| {
        let m =
            Mat4::rotation_y(0.5) * Mat4::translation(1.0, 2.0, 3.0) * Mat4::scale(2.0, 3.0, 4.0);
        b.iter(|| black_box(m.inverse()));
    });
}

criterion_group!(
    benches,
    bench_vec2_add,
    bench_vec2_mul,
    bench_vec3_normalize,
    bench_vec3_reflect,
    bench_mat2_transform,
    bench_mat2_batch_transform,
    bench_mat2_transform_in_place,
    bench_mat4_mul,
    bench_mat4_transform_point,
    bench_mat4_orthographic,
    bench_mat4_inverse
);
criterion_main!(benches);
