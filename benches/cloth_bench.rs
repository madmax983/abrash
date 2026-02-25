use abrash::experimental::cloth::Cloth;
use abrash::math::Vec3;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_cloth_update(c: &mut Criterion) {
    let width = 50;
    let height = 50;
    let spacing = 0.1;
    let mut cloth = Cloth::new(width, height, spacing);

    // Pin top corners
    cloth.pin(0, 0);
    cloth.pin(width - 1, 0);

    let gravity = Vec3::new(0.0, -9.8, 0.0);
    let wind = Vec3::new(1.0, 0.0, 1.0);
    let dt = 0.016;

    let mut group = c.benchmark_group("cloth");
    group.bench_function("update_50x50", |b| {
        b.iter(|| {
            cloth.update(dt, gravity, wind);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_cloth_update);
criterion_main!(benches);
