use abrash::math::{Mat4, Vec3};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

// Scalar implementation for comparison
fn transform_point_scalar_impl(m: &Mat4, v: Vec3) -> (Vec3, f32) {
    let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z + m.m[3][0];
    let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z + m.m[3][1];
    let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z + m.m[3][2];
    let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z + m.m[3][3];
    (Vec3::new(x, y, z), w)
}

fn bench_transform_point_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_point_single");

    let m = Mat4::rotation_y(0.5) * Mat4::translation(10.0, 5.0, 2.0);
    let v = Vec3::new(1.0, 2.0, 3.0);

    group.bench_function("simd_optimized", |b| {
        b.iter(|| {
            m.transform_point(black_box(v))
        });
    });

    group.bench_function("scalar_baseline", |b| {
        b.iter(|| {
            transform_point_scalar_impl(black_box(&m), black_box(v))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_transform_point_single);
criterion_main!(benches);
