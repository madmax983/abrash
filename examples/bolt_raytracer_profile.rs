//! Deterministic profiling harness for `RayTracer::render` (recursive Whitted-style
//! raytracer: primary rays, hard shadows, 3 reflection bounces).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This binary
//! is meant to be run under `valgrind --tool=callgrind` / `--tool=dhat` to get
//! deterministic instruction and allocation counts for a realistic workload that
//! mirrors `examples/raytracer_demo.rs` exactly: 5 cube objects (floor + 4 cubes,
//! 12 triangles each = 60 triangles total), 400x300 resolution, one object animated
//! every frame (rotation + bounce) so no frame is constant-foldable.
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash_render::experimental::raytracer::RayTracer;
use std::f32::consts::PI;
use std::hint::black_box;
use std::sync::Arc;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

fn build_scene() -> Scene {
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let eye = Vec3::new(0.0, 3.0, 6.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let camera = Camera::new(view, projection);
    let mut scene = Scene::new(camera);

    let cube_mesh = Arc::new(Mesh::cube(1.0));

    let floor_transform = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(10.0, 0.1, 10.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        floor_transform,
        0xFF40_4040,
    ));
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        Mat4::translation(0.0, 0.0, 0.0),
        0xFFFF_0000,
    ));
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        Mat4::translation(-2.5, 0.0, -1.0) * Mat4::rotation_y(PI / 4.0),
        0xFF00_FF00,
    ));
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        Mat4::translation(2.5, 0.0, -1.0) * Mat4::rotation_y(-PI / 4.0),
        0xFF00_00FF,
    ));
    scene.add_object(SceneObject::new(
        cube_mesh,
        Mat4::translation(0.0, 0.0, 2.5) * Mat4::scale(0.5, 0.5, 0.5),
        0xFFFF_FFFF,
    ));

    scene
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let mut scene = build_scene();
    let mut renderer = RayTracer::new();
    renderer.max_bounces = 3;
    renderer.background_color = 0xFF10_1015;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();

    let mut time = 0.0f32;
    for _ in 0..frames {
        time += 0.01;
        // Mirrors raytracer_demo.rs's per-frame update: rotate + bob the center cube.
        let rotation = Mat4::rotation_y(time);
        let translation = Mat4::translation(0.0, 0.5 + (time * 2.0).sin() * 0.5, 0.0);
        scene.objects[1].transform = translation * rotation;

        renderer.render(&scene, &mut fb);
        black_box(&fb);
    }

    // Checksum to keep the compiler from eliding the render work, and to compare
    // before/after pixel output for behavior-preservation.
    let checksum: u64 = fb.as_slice().iter().map(|&p| u64::from(p)).sum();
    println!("frames={frames} checksum={checksum}");
}
