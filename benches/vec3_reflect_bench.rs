use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn bench_vec3_reflect(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_reflect");

    let v = Vec3::new(1.0, 2.0, 3.0);
    let n = Vec3::new(0.0, 1.0, 0.0);

    group.bench_function("reflect", |b| {
        b.iter(|| {
            black_box(v.reflect(n));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_vec3_reflect);
criterion_main!(benches);
