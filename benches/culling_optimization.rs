use abrash::culling::Frustum;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::AABB;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_frustum_culling_optimizations(c: &mut Criterion) {
    // Setup Frustum
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let vp = view * proj;
    let frustum = Frustum::from_matrix(vp);

    // Create 10,000 equivalent AABBs
    let mut aabbs = Vec::with_capacity(10_000);

    for i in 0..10_000 {
        let x = ((i % 100) as f32) - 50.0;
        let y = ((i / 100) as f32) - 50.0;
        let z = 0.0;
        let center = Vec3::new(x, y, z);

        aabbs.push(AABB::new(
            center - Vec3::new(0.5, 0.5, 0.5),
            center + Vec3::new(0.5, 0.5, 0.5),
        ));
    }

    let mut group = c.benchmark_group("culling_optimizations");

    // Optimization: AABB culling
    group.bench_function("cull_aabbs_prealloc", |b| {
        let mut results = vec![true; aabbs.len()];
        b.iter(|| {
            frustum.cull_aabbs_prealloc(black_box(&aabbs), black_box(&mut results));
            black_box(&results);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_frustum_culling_optimizations);
criterion_main!(benches);
