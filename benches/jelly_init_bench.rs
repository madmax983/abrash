use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize};
use abrash_render::experimental::jelly::SoftBody;
use abrash_core::mesh::Mesh;
use abrash_core::math::Vec3;

fn bench_jelly_init(c: &mut Criterion) {
    let mut mesh = Mesh::new();
    let n = 50;
    for i in 0..n {
        for j in 0..n {
            mesh.vertices.push(Vec3::new(i as f32, j as f32, 0.0));
        }
    }
    for i in 0..n - 1 {
        for j in 0..n - 1 {
            let top_left = i * n + j;
            let top_right = i * n + j + 1;
            let bottom_left = (i + 1) * n + j;
            let bottom_right = (i + 1) * n + j + 1;
            mesh.indices.push([top_left, top_right, bottom_left]);
            mesh.indices.push([top_right, bottom_right, bottom_left]);
        }
    }

    c.bench_function("jelly_init_50x50", |b| {
        b.iter_batched(|| mesh.clone(), |m| {
            SoftBody::new(black_box(m), 1.0, 1.0, 1.0).unwrap()
        }, BatchSize::SmallInput);
    });
}

criterion_group!(benches, bench_jelly_init);
criterion_main!(benches);
