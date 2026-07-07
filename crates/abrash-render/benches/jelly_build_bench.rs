use abrash_core::mesh::Mesh;
use abrash_render::experimental::jelly::SoftBody;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn jelly_build_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("jelly_build");

    let mesh = Mesh::sphere(1.0, 32, 32);

    group.bench_function("SoftBody::new", |b| {
        b.iter(|| {
            let _body = SoftBody::new(black_box(mesh.clone()), 1.0, 1.0, 1.0).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, jelly_build_benchmark);
criterion_main!(benches);
