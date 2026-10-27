//! Jump-flood SDF + irradiance probe demo: rasterize an analytic scene
//! into occupancy, rebuild a signed distance field with the jump flood
//! algorithm, then light it with sphere-traced soft shadows from a point
//! light plus probe-gathered bounce.
//!
//! Run with: `cargo run -p abrash-core --example jump_flood_demo`
//! Writes `jump_flood_demo.ppm` to the current directory.

use std::time::Instant;

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::jump_flood::{JumpFloodSdf2D, ProbeGrid2D, occupancy_from_sdf_2d};
use abrash_core::math::Vec2;
use abrash_core::sdf::{circle_2d, rect_2d};

const FIELD: u32 = 96;

fn scene_sdf(p: Vec2) -> f32 {
    let a = circle_2d(p, Vec2::new(62.0, 34.0), 13.0);
    let b = rect_2d(p, Vec2::new(30.0, 64.0), Vec2::new(11.0, 9.0));
    let c = circle_2d(p, Vec2::new(72.0, 72.0), 8.0);
    a.min(b).min(c)
}

fn main() {
    // 1. Occupancy from the analytic scene, then the jump-flood field.
    let occ = occupancy_from_sdf_2d(FIELD, FIELD, scene_sdf);
    let solid = occ.iter().filter(|c| **c).count();
    let started = Instant::now();
    let field = JumpFloodSdf2D::from_occupancy(FIELD, FIELD, &occ);
    let build_us = started.elapsed().as_micros();
    println!(
        "jump_flood_demo: {FIELD}x{FIELD} field, {solid} solid cells, JFA rebuild {build_us} µs"
    );
    println!(
        "centre-of-circle distance {:.2} (expect ≈ -12.5), far corner {:.2}",
        field.signed_distance(62, 34),
        field.signed_distance(2, 2)
    );

    // 2. Probes: warm walls bounce a dim blue sky.
    let light = Vec2::new(18.0, 18.0);
    let wall_base = Color::rgb(0.85, 0.62, 0.38);
    let mut probes = ProbeGrid2D::new(Vec2::new(4.0, 4.0), 8.0, 12, 12, 32);
    probes.update(
        &field,
        140.0,
        Color::rgb(0.10, 0.14, 0.24),
        &|point, normal| {
            // Cheap Lambert wrap toward the light, evaluated by the caller —
            // the probe system itself only transports radiance.
            let to_light = (light - point).length();
            let dir = if to_light > 1e-4 {
                (light - point) * (1.0 / to_light)
            } else {
                Vec2::ZERO
            };
            let wrap = (normal.dot(dir) * 0.5 + 0.5).clamp(0.0, 1.0);
            let atten = 900.0 / (900.0 + to_light * to_light);
            wall_base * (0.15 + 0.85 * wrap * atten)
        },
    );
    let centre_probe = probes.probe(6, 6).expect("centre probe");
    println!(
        "centre probe irradiance ({:.3}, {:.3}, {:.3}), mean hit distance {:.1}",
        centre_probe.irradiance.r,
        centre_probe.irradiance.g,
        centre_probe.irradiance.b,
        centre_probe.mean_distance
    );

    // 3. Shadow sanity: one lit point, one point behind the circle.
    let lit = field.shadow_visibility(Vec2::new(30.0, 20.0), light, 12.0);
    let shaded = field.shadow_visibility(Vec2::new(84.0, 44.0), light, 12.0);
    let penumbra = field.shadow_visibility(Vec2::new(66.0, 54.0), light, 12.0);
    println!("shadow samples: lit {lit:.3}, behind circle {shaded:.3}, grazing {penumbra:.3}");

    // 4. Render at 2× the field resolution.
    let scale = 2u32;
    let (w, h) = (FIELD * scale, FIELD * scale);
    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    for y in 0..h {
        for x in 0..w {
            let p = Vec2::new(x as f32 / scale as f32, y as f32 / scale as f32);
            let d = field.sample(p);
            let color = if d <= 0.0 {
                wall_base * 0.55
            } else {
                let vis = field.shadow_visibility(p, light, 12.0);
                let bounce = probes.sample(p);
                let direct = Color::rgb(1.0, 0.92, 0.78) * (vis * 0.75);
                (direct + bounce * 0.55).saturate()
            };
            fb.set_pixel(x as i32, y as i32, color.to_argb_u32());
        }
    }
    // Light marker.
    for dy in -2..=2i32 {
        for dx in -2..=2i32 {
            fb.set_pixel(
                (light.x as i32 * 2) + dx,
                (light.y as i32 * 2) + dy,
                0xFFFF_FF99,
            );
        }
    }
    fb.export_ppm("jump_flood_demo.ppm")
        .expect("writing jump_flood_demo.ppm");
    println!("wrote jump_flood_demo.ppm ({w}x{h})");
}
