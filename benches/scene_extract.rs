use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::math::{Mat4, Vec3};
use abrash_render::scene::{Scene, SceneObject, Camera};
use abrash_core::mesh::Mesh;
use std::sync::Arc;

fn bench_scene_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene_extract");

    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 50.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 1000.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let mesh = Arc::new(Mesh::cube(1.0));
    for i in 0..1000 {
        let x = (i % 10) as f32 * 2.0 - 10.0;
        let y = (i / 10 % 10) as f32 * 2.0 - 10.0;
        let z = (i / 100) as f32 * 2.0 - 10.0;
        let transform = Mat4::translation(x, y, z);
        scene.add_object(SceneObject::new(mesh.clone(), transform, 0xFFFFFFFF));
    }

    group.bench_function("1000 cubes", |b| b.iter(|| {
        black_box(scene.extract());
    }));

    group.finish();
}

criterion_group!(benches, bench_scene_extract);
criterion_main!(benches);
