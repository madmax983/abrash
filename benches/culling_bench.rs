use abrash::culling::Frustum;
use abrash::geometry::BoundingSphere;
use abrash::math::Mat4;
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_cull_spheres(c: &mut Criterion) {
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let frustum = Frustum::from_matrix(proj);

    let mut spheres = Vec::new();
    for i in 0..1000 {
        spheres.push(BoundingSphere {
            center: Vec3::new(0.0, 0.0, i as f32),
            radius: 1.0,
        });
    }

    c.bench_function("cull_spheres", |b| {
        b.iter(|| black_box(frustum.cull_spheres(black_box(&spheres))));
    });
}

criterion_group!(benches, bench_cull_spheres);
criterion_main!(benches);
