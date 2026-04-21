use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::geometry::{Cone, Ray};
use abrash_core::math::Vec3;

fn bench_cone_intersects_ray(c: &mut Criterion) {
    let mut group = c.benchmark_group("cone");

    let cone = Cone::from_radius_height(Vec3::new(1.0, 2.0, 3.0), 5.0, 10.0);
    let ray = Ray::new(Vec3::new(10.0, 5.0, 10.0), Vec3::new(-1.0, -0.5, -1.0).normalize());

    group.bench_function("intersects_ray", |b| {
        b.iter(|| {
            black_box(cone.intersects_ray(black_box(&ray)))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_cone_intersects_ray);
criterion_main!(benches);
