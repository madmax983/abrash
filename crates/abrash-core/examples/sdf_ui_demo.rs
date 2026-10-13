//! Analytic SDF UI demo: a panel, a pill button, and a ring avatar,
//! rendered with per-pixel SDF coverage (`smoothstep(fwidth, -fwidth, d)`)
//! into a PPM — MSAA-quality edges, no multisample buffer.
//!
//! Run with: `cargo run -p abrash-core --example sdf_ui_demo`
//! Writes `sdf_ui_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_core::sdf::{capsule_2d, circle_2d, rounded_rect_2d};
use abrash_core::sdf_ui::{fill_sdf_shape, stroke_sdf_shape};

fn main() {
    let (w, h) = (320, 180);
    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    fb.clear(0xFF10_1018);

    // Panel.
    fill_sdf_shape(&mut fb, None, Color::grey(0.22), |p| {
        rounded_rect_2d(p, Vec2::new(160.0, 90.0), Vec2::new(140.0, 70.0), 16.0)
    });
    // Panel border (stroke of the same contour).
    stroke_sdf_shape(&mut fb, None, Color::grey(0.55), 1.0, |p| {
        rounded_rect_2d(p, Vec2::new(160.0, 90.0), Vec2::new(140.0, 70.0), 16.0)
    });
    // Avatar: filled circle + ring.
    fill_sdf_shape(&mut fb, None, Color::rgb(0.2, 0.45, 0.9), |p| {
        circle_2d(p, Vec2::new(64.0, 64.0), 26.0)
    });
    stroke_sdf_shape(&mut fb, None, Color::WHITE, 2.0, |p| {
        circle_2d(p, Vec2::new(64.0, 64.0), 26.0)
    });
    // Pill button.
    fill_sdf_shape(&mut fb, None, Color::rgb(0.15, 0.75, 0.35), |p| {
        capsule_2d(p, Vec2::new(120.0, 128.0), Vec2::new(200.0, 128.0), 16.0)
    });

    let partial = fb
        .as_slice()
        .iter()
        .filter(|&&px| px != 0xFF10_1018)
        .count();
    fb.export_ppm("sdf_ui_demo.ppm").expect("write PPM");
    println!("sdf_ui_demo: {w}x{h}, {partial} non-background pixels, wrote sdf_ui_demo.ppm");
}
