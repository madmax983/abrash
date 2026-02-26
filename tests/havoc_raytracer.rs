#![cfg(feature = "nova")]

use abrash::experimental::raytracer::RayTracer;
use abrash::scene::{Scene, SceneObject, Camera};
use abrash::mesh::Mesh;
use abrash::math::{Mat4, Vec3};
use abrash::framebuffer::Framebuffer;
use std::sync::Arc;

#[test]
fn test_stack_overflow_recursion() {
    // "You assumed the user wouldn't ask for 1,000,000 bounces."
    // This setting forces the recursive raytracer to dive 1,000,000 frames deep.
    // Given a standard stack size (2-8MB) and a non-zero stack frame size,
    // this WILL cause a Stack Overflow / SIGSEGV.
    let mut tracer = RayTracer::new();
    tracer.max_bounces = 1_000_000;

    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Camera at Origin, looking down -Z
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0)
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let camera = Camera::new(view, proj);

    let mut scene = Scene::new(camera);

    // Create an "Infinity Mirror" setup
    let mesh = Arc::new(Mesh::cube(10.0)); // 10x10x10 cube

    // Mirror 1 (Front): Located at Z=-10. Front face is at Z=-5.
    // Ray (0,0,0 -> 0,0,-1) hits it. Reflected normal (0,0,1) -> ray goes (0,0,1).
    let obj1 = SceneObject::new(mesh.clone(), Mat4::translation(0.0, 0.0, -10.0), 0xFFFFFFFF);

    // Mirror 2 (Back): Located at Z=10. Front face is at Z=5.
    // Ray (0,0,1) hits it. Reflected normal (0,0,-1) -> ray goes (0,0,-1).
    let obj2 = SceneObject::new(mesh, Mat4::translation(0.0, 0.0, 10.0), 0xFFFFFFFF);

    scene.add_object(obj1);
    scene.add_object(obj2);

    // This should crash the test runner with a stack overflow.
    tracer.render(&scene, &mut fb);
}
