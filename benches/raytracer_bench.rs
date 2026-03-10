use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::experimental::raytracer::RayTracer;
use abrash::scene::{Camera, Scene, SceneObject};
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;

fn bench_raytracer(c: &mut Criterion) {
    let width = 64;
    let height = 64;

    let mesh = Arc::new(Mesh::cube(10.0));

    let view = Mat4::look_at(
        Vec3::new(0.0, 50.0, 50.0), // Eye
        Vec3::new(0.0, 0.0, 0.0),   // Target
        Vec3::new(0.0, 1.0, 0.0),   // Up
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 1000.0);
    let camera = Camera::new(view, proj);

    let mut scene = Scene::new(camera);

    // Add objects
    for i in 0..10 {
        let x = (i % 5) as f32 * 15.0 - 75.0;
        let z = (i / 5) as f32 * 15.0 - 75.0;
        let transform = Mat4::translation(x, 0.0, z);
        let object = SceneObject::new(mesh.clone(), transform, 0xFFFFFFFF);
        scene.add_object(object);
    }

    let tracer = RayTracer::new();
    let mut fb = Framebuffer::new(width, height).unwrap();

    c.bench_function("raytracer_render_10_objects_64x64", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            tracer.render(&scene, &mut fb);
        });
    });
}

criterion_group!(benches, bench_raytracer);
criterion_main!(benches);
