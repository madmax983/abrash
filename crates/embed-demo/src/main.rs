//! Demo binary: renders a spinning cube offscreen and exports the final frame as PPM.
//!
//! This binary has zero platform dependencies — it does not create a window, does not
//! use crossterm, ratatui, or ratzilla. It is the integration proof for ADR 005.
//!
//! Run with: `cargo run -p embed-demo`

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use embed_demo::{AbrashBackend, EmbedCamera, EmbedDraw, EmbedScene};
use std::f32::consts::FRAC_PI_3;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const FRAMES: u32 = 8;

fn main() {
    println!("abrash embed-demo: host-owned offscreen rendering without platform dependencies");
    println!("  Resolution : {WIDTH}×{HEIGHT}");
    println!("  Frames     : {FRAMES}");
    println!("  Path       : caller-owned pixel/depth buffers");

    let mut backend = AbrashBackend::new(WIDTH, HEIGHT);
    let len = (WIDTH as usize) * (HEIGHT as usize);
    let mut pixels = vec![0xFF00_0000; len];
    let mut depths = vec![f32::INFINITY; len];

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

        backend
            .render_into(
                &EmbedScene {
                    camera: camera.clone(),
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
                },
                pixels.as_mut_slice(),
                depths.as_mut_slice(),
            )
            .expect("render_into");

        let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
        let coverage = visible as f32 / pixels.len() as f32 * 100.0;
        println!("  Frame {frame_idx:>2}: {visible:>5} visible pixels ({coverage:.1}%)");
        last_visible = visible;
    }

    // Export the last host-owned color buffer as PPM so we have a concrete artifact.
    let out_path = "embed_demo_output.ppm";
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT).expect("valid framebuffer");
    framebuffer.as_mut_slice().copy_from_slice(&pixels);
    framebuffer.export_ppm(out_path).expect("PPM export");

    println!("\nExported last frame → {out_path}");
    println!("Final frame: {last_visible} non-background pixels");

    assert!(last_visible > 0, "at least one visible pixel expected");
    println!(
        "\nSeam purity proof: this binary linked with ZERO platform crates and used host-owned buffers."
    );
    println!("Run: cargo tree -p embed-demo --no-default-features");
}
