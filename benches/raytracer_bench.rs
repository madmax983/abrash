use abrash::experimental::raytracer::RayTracer;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::scene::{Camera, Scene, SceneObject};
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;

fn bench_raytracer(c: &mut Criterion) {
    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let mesh = Arc::new(Mesh::cube(2.0));
    let transform = Mat4::identity();
    scene.add_object(SceneObject::new(mesh, transform, 0xFFFFFFFF));

    let raytracer = RayTracer::new();

    c.bench_function("raytracer_render_64x64", |b| {
        b.iter(|| {
            raytracer.render(&scene, &mut fb);
        })
    });
}

criterion_group!(benches, bench_raytracer);
criterion_main!(benches);
