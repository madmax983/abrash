use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::raytracer::{RayTracer};
use abrash_core::math::{Vec3, Mat4};
use abrash_core::mesh::Mesh;
use abrash_render::scene::{Camera, Scene, SceneObject};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::sync::Arc;

fn benchmark_raytracer(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let tracer = RayTracer::default();

    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.57, 800.0/600.0, 0.1, 100.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let mut mesh = Mesh::new();
    mesh.vertices = vec![
        Vec3::new(-1.0, -1.0, 0.0),
        Vec3::new(1.0, -1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(-1.0, 1.0, 0.0),
    ];
    mesh.indices = vec![[0, 1, 2], [0, 2, 3]];
    scene.add_object(SceneObject::new(Arc::new(mesh), Mat4::identity(), 0xFFFF_0000));

    c.bench_function("raytracer_800x600", |b| {
        b.iter(|| tracer.render(black_box(&scene), black_box(&mut fb)));
    });
}

criterion_group!(benches, benchmark_raytracer);
criterion_main!(benches);
