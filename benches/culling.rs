use abrash_core::geometry::Frustum;
use abrash::geometry::{AABB, BoundingSphere};
use abrash::math::{Mat4, Vec3};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_culling(c: &mut Criterion) {
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let frustum = Frustum::from_view_projection(&(view * proj));

    let mut aabbs = Vec::new();
    let mut spheres = Vec::new();

    // Deterministic random generation for repeatable benchmarks
    let mut rng_seed = 12345u32;
    let mut rand_f32 = || {
        rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (rng_seed as f32) / (u32::MAX as f32)
    };

    for _ in 0..10000 {
        let x = rand_f32() * 40.0 - 20.0;
        let y = rand_f32() * 40.0 - 20.0;
        let z = rand_f32() * 40.0 - 20.0;
        let sx = rand_f32() * 4.9 + 0.1;
        let sy = rand_f32() * 4.9 + 0.1;
        let sz = rand_f32() * 4.9 + 0.1;

        aabbs.push(AABB::new(
            Vec3::new(x - sx, y - sy, z - sz),
            Vec3::new(x + sx, y + sy, z + sz),
        ));
        spheres.push(BoundingSphere {
            center: Vec3::new(x, y, z),
            radius: sx, // Rough approximation
        });
    }

    let mut results = vec![false; aabbs.len()];
    let transform =
        Mat4::translation(1.0, 2.0, 3.0) * Mat4::rotation_y(0.5) * Mat4::scale(2.0, 2.0, 2.0);

    let mut group = c.benchmark_group("culling");

    group.bench_function("frustum_cull_10k_aabbs_scalar", |b| {
        b.iter(|| {
            for (i, aabb) in aabbs.iter().enumerate() {
                results[i] = frustum.intersects_aabb(aabb);
            }
        });
    });

    group.bench_function("frustum_cull_10k_aabbs_simd", |b| {
        b.iter(|| {
            frustum.cull_aabbs_prealloc(&aabbs, &mut results);
        });
    });

    group.bench_function("frustum_cull_10k_spheres_scalar", |b| {
        b.iter(|| {
            for (i, sphere) in spheres.iter().enumerate() {
                results[i] = frustum.intersects_sphere(sphere.center, sphere.radius);
            }
        });
    });

    group.bench_function("frustum_cull_10k_spheres_simd", |b| {
        b.iter(|| {
            frustum.cull_spheres_prealloc(&spheres, &mut results);
        });
    });

    group.bench_function("aabb_transform", |b| {
        b.iter(|| {
            for aabb in &aabbs {
                let _ = aabb.transform(&transform);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_culling);
criterion_main!(benches);
