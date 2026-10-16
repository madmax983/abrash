//! SDF soft shadows + ambient occlusion demo.
//!
//! Ray-marches a small SDF scene (ground plane, a sphere, and a box) with
//! penumbra soft shadows (`soft_shadow_estimate`) and 5-tap SDF ambient
//! occlusion (`ambient_occlusion_estimate`) wired into the lighting.
//! Writes the result to `sdf_soft_shadow_demo.ppm`.
//!
//! Run with:
//! `cargo run -p abrash-render --release --example sdf_soft_shadow_demo`

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene, render_sdf};
use std::fs::File;
use std::io::Write;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    fb.clear(0xFF18_1A22);
    zb.clear();

    let mut scene = SdfScene::new();

    // Ground plane (y = 0).
    scene.add(SdfObject {
        primitive: SdfPrimitive::Plane {
            normal: Vec3::new(0.0, 1.0, 0.0),
            distance: 0.0,
        },
        color: 0xFF3A_5F3A,
    });

    // Sphere resting on the ground: casts a soft penumbra shadow.
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(0.0, 1.0, 0.0),
        },
        color: 0xFFC4_4B33,
    });

    // Box off to the side: second occluder, plus AO darkening where it
    // meets the ground.
    scene.add(SdfObject {
        primitive: SdfPrimitive::Box {
            size: Vec3::new(0.6, 0.6, 0.6),
            center: Vec3::new(2.4, 0.6, -0.6),
        },
        color: 0xFF3E_6FB0,
    });

    let camera_pos = Vec3::new(4.5, 3.2, 6.5);
    let view = Mat4::look_at(
        camera_pos,
        Vec3::new(0.0, 0.8, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);

    render_sdf(&mut fb, &mut zb, &scene, &view, &proj, camera_pos);

    let mut file = File::create("sdf_soft_shadow_demo.ppm").unwrap();
    write!(file, "P3\n{width} {height}\n255\n").unwrap();
    for pixel in fb.as_slice() {
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;
        writeln!(file, "{r} {g} {b}").unwrap();
    }

    println!("Saved sdf_soft_shadow_demo.ppm");
}
