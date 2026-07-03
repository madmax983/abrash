use criterion::{Criterion, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::experimental::raytracer::RayTracer;
use abrash_render::scene::{Camera, Scene, SceneObject};
use std::sync::Arc;

fn raytracer_benchmark(c: &mut Criterion) {
    let mut tracer = RayTracer::new();
    tracer.background_color = 0xFF00_0000;

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let mut fb = Framebuffer::new(64, 64).unwrap();

    let mut mesh = Mesh::new();
    mesh.vertices = vec![
        Vec3::new(-2.0, -2.0, 0.0),
        Vec3::new(2.0, -2.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
    ];
    mesh.indices = vec![[0, 1, 2]];

    let object = SceneObject::new(Arc::new(mesh), Mat4::identity(), 0xFF_FFFFFF);
    scene.objects.push(object);

    c.bench_function("raytracer_render 64x64", |b| {
        b.iter(|| {
            tracer.render(&scene, &mut fb);
        });
    });
}

criterion_group!(benches, raytracer_benchmark);
criterion_main!(benches);
