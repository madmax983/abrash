//! Visibility-buffer demo: rasterise a small scene's triangle IDs and
//! barycentrics first, then shade each visible pixel exactly once in a
//! resolve pass — and print the overdraw the visbuffer absorbed.
//!
//! Run with: `cargo run -p abrash-core --example visibility_buffer_demo`
//! Writes `visibility_buffer_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Vec2, Vec3};
use abrash_core::visibility_buffer::{
    ResolveParams, VisMaterial, VisTriangle, VisVertex, VisibilityBuffer,
};

fn vertex(x: f32, y: f32, depth: f32, normal: Vec3) -> VisVertex {
    VisVertex::screen(x, y, depth)
        .with_normal(normal)
        .with_uv(Vec2::new(x / 160.0, y / 120.0))
}

fn main() {
    let (w, h) = (160_u32, 120_u32);

    // A ground quad (two triangles, farther), a nearer red triangle
    // overlapping it, and a small emissive "lamp" triangle. In a forward
    // renderer the ground under the red triangle would be shaded and
    // then painted over; here it is never shaded at all.
    let ground_n = Vec3::new(0.0, -0.25, 1.0);
    let tris = vec![
        VisTriangle::new(
            [
                vertex(8.0, 30.0, 0.8, ground_n),
                vertex(152.0, 30.0, 0.8, ground_n),
                vertex(152.0, 112.0, 0.8, ground_n),
            ],
            0,
        ),
        VisTriangle::new(
            [
                vertex(8.0, 30.0, 0.8, ground_n),
                vertex(152.0, 112.0, 0.8, ground_n),
                vertex(8.0, 112.0, 0.8, ground_n),
            ],
            0,
        ),
        VisTriangle::new(
            [
                vertex(50.0, 85.0, 0.4, Vec3::new(0.0, 0.0, 1.0)),
                vertex(110.0, 85.0, 0.4, Vec3::new(0.0, 0.0, 1.0)),
                vertex(80.0, 25.0, 0.4, Vec3::new(0.0, 0.0, 1.0)),
            ],
            1,
        ),
        VisTriangle::new(
            [
                vertex(128.0, 52.0, 0.3, Vec3::new(0.0, 0.0, 1.0)),
                vertex(144.0, 52.0, 0.3, Vec3::new(0.0, 0.0, 1.0)),
                vertex(136.0, 38.0, 0.3, Vec3::new(0.0, 0.0, 1.0)),
            ],
            2,
        ),
    ];
    let materials = [
        VisMaterial::plain(Color::rgb(0.25, 0.45, 0.30)),
        VisMaterial::plain(Color::rgb(0.75, 0.20, 0.15)),
        VisMaterial {
            base_color: Color::BLACK,
            emissive: Color::rgb(1.0, 0.8, 0.3),
        },
    ];

    let mut vis = VisibilityBuffer::new(w, h).expect("visibility buffer dimensions are valid");
    let stats = vis.rasterize(&tris);
    println!(
        "visibility_buffer_demo: {} triangles ({} degenerate), {} fragments tested, {} written, {} pixels covered",
        stats.triangles_input,
        stats.triangles_degenerate,
        stats.fragments_tested,
        stats.fragments_written,
        stats.covered_pixels,
    );
    println!(
        "overdraw absorbed by the visibility buffer: {} fragments never shaded",
        stats.fragments_tested - stats.covered_pixels,
    );

    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    let shaded = vis.resolve(
        &tris,
        &materials,
        &ResolveParams {
            light_dir: Vec3::new(0.35, -0.45, 0.82),
            ambient: 0.2,
            background: 0xFF08_0C14,
        },
        &mut fb,
    );
    println!("resolve shaded {shaded} pixels exactly once");
    fb.export_ppm("visibility_buffer_demo.ppm")
        .expect("writing the demo PPM cannot fail");
    println!("wrote visibility_buffer_demo.ppm ({w}x{h})");
}
