use abrash::culling::Frustum;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::BoundingSphere;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_frustum_intersection(c: &mut Criterion) {
    // Setup Frustum
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let vp = view * proj;
    let frustum = Frustum::from_matrix(vp);

    // Create 10,000 spheres
    // Grid from -50 to 50 in X and Y, and spread in Z
    let mut spheres = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        let x = ((i % 100) as f32) - 50.0;
        let y = ((i / 100) as f32) - 50.0;
        let z = 0.0; // Place them on Z plane for simplicity, some will be culled by left/right/top/bottom
        spheres.push(BoundingSphere {
            center: Vec3::new(x, y, z),
            radius: 0.5,
        });
    }

    let mut group = c.benchmark_group("culling");

    group.bench_function("frustum_cull_10k_spheres_scalar", |b| {
        b.iter(|| {
            let mut visible_count = 0;
            for sphere in &spheres {
                if frustum.intersects(black_box(sphere)) {
                    visible_count += 1;
                }
            }
            black_box(visible_count);
        });
    });

    group.bench_function("frustum_cull_10k_spheres_simd", |b| {
        b.iter(|| {
            let results = frustum.cull_spheres(black_box(&spheres));
            let visible_count = results.iter().filter(|&&v| v).count();
            black_box(visible_count);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_frustum_intersection);
criterion_main!(benches);
