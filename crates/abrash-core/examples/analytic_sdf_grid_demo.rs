//! Analytic SDF grid demo: a baked sphere-and-box scene, ray-traced
//! twice — once with the analytic narrow-band cell solver, once with
//! classic sphere tracing — shading both from the analytic gradient
//! and reporting the work each path did.
//!
//! Run with: `cargo run -p abrash-core --example analytic_sdf_grid_demo`
//! Writes `analytic_sdf_grid_demo.ppm` to the current directory.

use abrash_core::analytic_sdf_grid::{SdfGrid, TraceParams};
use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;

const W: usize = 96;
const H: usize = 72;

/// Scene: a sphere of radius 2.2 at the origin, unioned with a box
/// centred at (1.6, −0.6, 0.4) with half extents 0.9.
fn scene(p: Vec3) -> f32 {
    let sphere = p.length() - 2.2;
    let q = Vec3::new(
        (p.x - 1.6).abs() - 0.9,
        (p.y + 0.6).abs() - 0.9,
        (p.z - 0.4).abs() - 0.9,
    );
    let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
    let inside = q.x.max(q.y).max(q.z).min(0.0);
    sphere.min(outside + inside)
}

fn main() {
    // 25³ nodes, spacing 0.4: the field covers [−4.8, 4.8]³ in 61 KB.
    let grid = SdfGrid::bake(25, 25, 25, Vec3::splat(-4.8), 0.4, scene).expect("grid bakes");
    println!(
        "analytic_sdf_grid_demo: baked 25x25x25 grid, {} bytes of samples",
        grid.memory_bytes()
    );

    let params = TraceParams::default();
    let light = Vec3::new(-0.5, 0.8, -0.35).normalize_or_zero();
    let mut fb = Framebuffer::new(W as u32, H as u32).expect("framebuffer dimensions are valid");
    let (mut hits, mut cells, mut skipped, mut solved, mut samples) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    for y in 0..H {
        for x in 0..W {
            // Orthographic rays down +z, spread over the grid's face.
            let ro = Vec3::new(
                -4.8 + 9.6 * (x as f32 + 0.5) / W as f32,
                4.8 - 9.6 * (y as f32 + 0.5) / H as f32,
                -8.0,
            );
            let analytic = grid.intersect(ro, Vec3::Z, &params);
            let marched = grid.sphere_trace(ro, Vec3::Z, &params);
            cells += analytic.cells_visited;
            skipped += analytic.cells_skipped;
            solved += analytic.cells_solved;
            samples += marched.field_samples;
            assert_eq!(
                analytic.hit.is_some(),
                marched.hit.is_some(),
                "analytic and marched disagree at pixel ({x}, {y})"
            );
            let i = y * W + x;
            if let Some(hit) = analytic.hit {
                hits += 1;
                let shade = 0.25 + 0.75 * hit.normal.dot(light).max(0.0);
                fb.as_mut_slice()[i] =
                    Color::rgb(0.85 * shade, 0.45 * shade, 0.20 * shade).to_argb_u32();
            } else {
                fb.as_mut_slice()[i] = Color::rgb(0.05, 0.08, 0.16).to_argb_u32();
            }
        }
    }
    println!(
        "analytic: {hits} hits over {} rays — {cells} cells visited, {skipped} skipped by the band, {solved} cubics solved, 0 field samples",
        W * H
    );
    println!("sphere-traced baseline: {samples} field samples for the same image");
    fb.export_ppm("analytic_sdf_grid_demo.ppm")
        .expect("writing the demo PPM cannot fail");
    println!("wrote analytic_sdf_grid_demo.ppm ({W}x{H})");
}
