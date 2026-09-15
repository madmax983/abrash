//! Deterministic profiling harness for the full scene-render pipeline
//! (culling + MVP transform + tile rasterization + framebuffer/z-buffer clear).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This binary
//! is meant to be run under `valgrind --tool=callgrind` / `--tool=dhat` to get
//! deterministic instruction and allocation counts for a realistic workload (100
//! objects, 20,000 triangles, 640x480), matching `benches/scene_render.rs`.
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use std::hint::black_box;
use std::sync::Arc;

fn generate_grid_mesh(size: usize) -> Mesh {
    let mut mesh = Mesh::new();
    let offset = size as f32 * 0.5;

    for y in 0..=size {
        for x in 0..=size {
            mesh.vertices
                .push(Vec3::new(x as f32 - offset, 0.0, y as f32 - offset));
        }
    }

    for y in 0..size {
        for x in 0..size {
            let i0 = y * (size + 1) + x;
            let i1 = i0 + 1;
            let i2 = (y + 1) * (size + 1) + x;
            let i3 = i2 + 1;

            mesh.indices.push([i0, i1, i2]);
            mesh.indices.push([i1, i3, i2]);
        }
    }

    mesh
}

fn build_scene(width: u32, height: u32) -> Scene {
    let mesh = Arc::new(generate_grid_mesh(10));
    let view = Mat4::look_at(
        Vec3::new(0.0, 50.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 1000.0);
    let mut scene = Scene::with_capacity(Camera::new(view, proj), 100);
    for i in 0..100 {
        let x = (i % 10) as f32 * 15.0 - 75.0;
        let z = (i / 10) as f32 * 15.0 - 75.0;
        scene.add_object(SceneObject::new(
            Arc::clone(&mesh),
            Mat4::translation(x, 0.0, z),
            0xFFFF_FFFF,
        ));
    }
    scene
}

fn main() {
    let width = 640;
    let height = 480;

    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let scene = build_scene(width, height);
    let mut renderer = TileRenderer::new(width, height);
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    for _ in 0..frames {
        fb.clear(0xFF00_0000);
        zb.clear();
        scene.render(&mut renderer, &mut fb, &mut zb);
        black_box(&fb);
    }

    // Checksum to keep the compiler from eliding the render work.
    let checksum: u64 = fb.as_slice().iter().map(|&p| p as u64).sum();
    println!("frames={frames} checksum={checksum}");
}
