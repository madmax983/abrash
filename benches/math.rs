use abrash::geometry::AABB;
use abrash::math::{Mat2, Mat3, Mat4, Vec2, Vec3, Vec4, fast_atan2, lerp, smoothstep};
use abrash::plane::Frustum;
use abrash::quat::Quat;
use abrash::ray::Ray;
use abrash::transform::Transform;
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

fn bench_vec3_fast_normalize(c: &mut Criterion) {
    c.bench_function("vec3_fast_normalize", |b| {
        let v = Vec3::new(1.0, 2.0, 3.0);
        b.iter(|| black_box(v.fast_normalize()));
    });
}

fn bench_sin_cos_separate(c: &mut Criterion) {
    c.bench_function("sin_cos_separate", |b| {
        let angle: f32 = 0.785;
        b.iter(|| {
            let sin = black_box(angle).sin();
            let cos = black_box(angle).cos();
            black_box((sin, cos))
        });
    });
}

fn bench_sin_cos_combined(c: &mut Criterion) {
    c.bench_function("sin_cos_combined", |b| {
        let angle: f32 = 0.785;
        b.iter(|| black_box(black_box(angle).sin_cos()));
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

fn bench_transform_to_mat4(c: &mut Criterion) {
    c.bench_function("transform_to_mat4", |b| {
        let transform = Transform::new(
            Vec3::new(2.0, -3.0, 4.0),
            Quat::from_euler(0.3, -0.7, 0.2),
            Vec3::new(1.5, 0.75, 2.0),
        );
        b.iter(|| black_box(transform.to_mat4()));
    });
}

fn bench_transform_point_direct(c: &mut Criterion) {
    c.bench_function("transform_point_direct", |b| {
        let transform = Transform::new(
            Vec3::new(2.0, -3.0, 4.0),
            Quat::from_euler(0.3, -0.7, 0.2),
            Vec3::new(1.5, 0.75, 2.0),
        );
        let point = Vec3::new(1.0, -2.0, 0.5);
        b.iter(|| black_box(transform.transform_point(point)));
    });
}

fn bench_transform_point_via_matrix(c: &mut Criterion) {
    c.bench_function("transform_point_via_matrix", |b| {
        let transform = Transform::new(
            Vec3::new(2.0, -3.0, 4.0),
            Quat::from_euler(0.3, -0.7, 0.2),
            Vec3::new(1.5, 0.75, 2.0),
        );
        let point = Vec3::new(1.0, -2.0, 0.5);
        b.iter(|| black_box(transform.to_mat4().transform_point(point).0));
    });
}

fn bench_fast_atan2(c: &mut Criterion) {
    c.bench_function("fast_atan2", |b| {
        b.iter(|| black_box(fast_atan2(black_box(0.866), black_box(0.5))));
    });
}

fn bench_std_atan2(c: &mut Criterion) {
    c.bench_function("std_atan2", |b| {
        b.iter(|| black_box(black_box(0.866_f32).atan2(black_box(0.5))));
    });
}

fn bench_smoothstep(c: &mut Criterion) {
    c.bench_function("smoothstep", |b| {
        b.iter(|| black_box(smoothstep(black_box(0.0), black_box(1.0), black_box(0.7))));
    });
}

fn bench_lerp_scalar(c: &mut Criterion) {
    c.bench_function("lerp_scalar", |b| {
        b.iter(|| black_box(lerp(black_box(0.0), black_box(10.0), black_box(0.5))));
    });
}

fn bench_mat3_transform(c: &mut Criterion) {
    c.bench_function("mat3_transform", |b| {
        let m = Mat3::rotation_y(0.785);
        let v = Vec3::new(1.0, 2.0, 3.0);
        b.iter(|| black_box(m.transform(black_box(v))));
    });
}

fn bench_mat3_inverse(c: &mut Criterion) {
    c.bench_function("mat3_inverse", |b| {
        let m = Mat3::rotation_y(0.5) * Mat3::scale(2.0, 1.5, 0.5);
        b.iter(|| black_box(m.inverse()));
    });
}

fn bench_mat3_inverse_transpose(c: &mut Criterion) {
    c.bench_function("mat3_inverse_transpose", |b| {
        let m = Mat3::rotation_y(0.5) * Mat3::scale(2.0, 1.5, 0.5);
        b.iter(|| black_box(m.inverse_transpose()));
    });
}

fn bench_vec4_normalize(c: &mut Criterion) {
    c.bench_function("vec4_normalize", |b| {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        b.iter(|| black_box(v.normalize()));
    });
}

fn bench_ray_intersect_aabb(c: &mut Criterion) {
    c.bench_function("ray_intersect_aabb", |b| {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        b.iter(|| black_box(ray.intersect_aabb(black_box(&aabb))));
    });
}

fn bench_ray_intersect_triangle(c: &mut Criterion) {
    c.bench_function("ray_intersect_triangle", |b| {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        b.iter(|| black_box(ray.intersect_triangle(black_box(v0), black_box(v1), black_box(v2))));
    });
}

fn bench_frustum_contains_aabb(c: &mut Criterion) {
    use std::f32::consts::PI;
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::UP);
    let proj = Mat4::perspective(PI / 2.0, 1.0, 1.0, 50.0);
    let frustum = Frustum::from_mat4(view * proj);
    let aabb = AABB::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));

    c.bench_function("frustum_contains_aabb", |b| {
        b.iter(|| black_box(frustum.contains_aabb(black_box(&aabb))));
    });
}

fn bench_frustum_contains_sphere(c: &mut Criterion) {
    use std::f32::consts::PI;
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::UP);
    let proj = Mat4::perspective(PI / 2.0, 1.0, 1.0, 50.0);
    let frustum = Frustum::from_mat4(view * proj);

    c.bench_function("frustum_contains_sphere", |b| {
        b.iter(|| black_box(frustum.contains_sphere(black_box(Vec3::ZERO), black_box(1.0))));
    });
}

criterion_group!(
    benches,
    bench_vec2_add,
    bench_vec2_mul,
    bench_vec3_normalize,
    bench_vec3_fast_normalize,
    bench_sin_cos_separate,
    bench_sin_cos_combined,
    bench_vec3_reflect,
    bench_mat2_transform,
    bench_mat2_batch_transform,
    bench_mat2_transform_in_place,
    bench_mat4_mul,
    bench_mat4_transform_point,
    bench_mat4_orthographic,
    bench_mat4_inverse,
    bench_transform_to_mat4,
    bench_transform_point_direct,
    bench_transform_point_via_matrix,
    bench_fast_atan2,
    bench_std_atan2,
    bench_smoothstep,
    bench_lerp_scalar,
    bench_mat3_transform,
    bench_mat3_inverse,
    bench_mat3_inverse_transpose,
    bench_vec4_normalize,
    bench_ray_intersect_aabb,
    bench_ray_intersect_triangle,
    bench_frustum_contains_aabb,
    bench_frustum_contains_sphere
);
criterion_main!(benches);
