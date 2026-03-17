//! Demo binary: renders a spinning cube offscreen and exports the final frame as PPM.
//!
//! This binary has zero platform dependencies — it does not create a window, does not
//! use crossterm, ratatui, or ratzilla. It is the integration proof for ADR 005.
//!
//! Run with: `cargo run -p embed-demo`

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use embed_demo::{AbrashBackend, EmbedCamera, EmbedDraw, EmbedScene};
use std::f32::consts::FRAC_PI_3;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const FRAMES: u32 = 8;

fn main() {
    println!("abrash embed-demo: offscreen rendering without platform dependencies");
    println!("  Resolution : {WIDTH}×{HEIGHT}");
    println!("  Frames     : {FRAMES}");

    let mut backend = AbrashBackend::new(WIDTH, HEIGHT);

    // Register geometry once — handles survive across frames.
    let cube = backend.register_mesh(&Mesh::cube(1.0));

    let camera = EmbedCamera {
        position: Vec3::new(0.0, 2.0, 5.0),
        target: Vec3::ZERO,
        fov_y: FRAC_PI_3,
    };

    let mut last_visible = 0usize;

    for frame_idx in 0..FRAMES {
        let angle = frame_idx as f32 * (std::f32::consts::TAU / FRAMES as f32);

        let pixels = backend.render(&EmbedScene {
            camera: EmbedCamera {
                position: camera.position,
                target: camera.target,
                fov_y: camera.fov_y,
            },
            draws: &[
                EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::rotation_y(angle),
                    color: 0xFFFF_4444, // red cube
                },
                EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::translation(2.5, 0.0, 0.0) * Mat4::rotation_y(-angle),
                    color: 0xFF44_44FF, // blue cube
                },
            ],
        });

        let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
        let coverage = visible as f32 / pixels.len() as f32 * 100.0;
        println!("  Frame {frame_idx:>2}: {visible:>5} visible pixels ({coverage:.1}%)");
        last_visible = visible;
    }

    // Export the last frame as PPM so we have a concrete artifact.
    let out_path = "embed_demo_output.ppm";
    backend
        .target()
        .framebuffer()
        .export_ppm(out_path)
        .expect("PPM export");

    println!("\nExported last frame → {out_path}");
    println!("Final frame: {last_visible} non-background pixels");

    assert!(last_visible > 0, "at least one visible pixel expected");
    println!("\nSeam purity proof: this binary linked with ZERO platform crates.");
    println!("Run: cargo tree -p embed-demo --no-default-features");
}
