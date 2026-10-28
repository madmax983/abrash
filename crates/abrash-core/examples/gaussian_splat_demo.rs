//! Gaussian-splat demo: a soft glowing "planet" with a tilted ring,
//! built from ~500 anisotropic splats and rendered by the CPU splat
//! pipeline (project → depth sort → front-to-back blend).
//!
//! Run with: `cargo run -p abrash-core --example gaussian_splat_demo`
//! Writes `gaussian_splat_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::gaussian_splat::{
    Gaussian, SplatCamera, SplatRenderParams, render_gaussians_to_framebuffer,
};
use abrash_core::math::Vec3;
use abrash_core::quat::Quat;
use std::f32::consts::PI;

/// Deterministic xorshift so the scene is the same on every run.
struct Rng(u32);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 10_000) as f32 / 10_000.0
    }
}

fn main() {
    let (w, h) = (320_u32, 180_u32);
    let mut rng = Rng(0x5EED_1234);
    let mut splats = Vec::new();

    // Planet body: splats scattered in a disc, denser and brighter
    // toward the middle, warm rim fading to a deep blue limb. Slightly
    // squashed in z so the depth sort has something to chew on.
    for _ in 0..420 {
        let angle = rng.next_f32() * 2.0 * PI;
        let radius = rng.next_f32().sqrt() * 1.05;
        let (x, y) = (angle.cos() * radius, angle.sin() * radius * 0.92);
        let z = (1.0 - radius * radius).max(0.0).sqrt() * 0.45 * (rng.next_f32() - 0.5);
        let heat = (1.0 - radius).clamp(0.0, 1.0);
        let color = Color::rgb(
            0.25 + 0.75 * heat,
            0.30 + 0.45 * heat * heat,
            0.85 - 0.45 * heat,
        );
        splats.push(Gaussian::isotropic(
            Vec3::new(x, y, z),
            0.16 + 0.10 * rng.next_f32(),
            color,
            0.55,
        ));
    }

    // A tilted ring of small, stretched splats around the planet,
    // each one tangent to the ring so the anisotropy reads as motion.
    for i in 0..90 {
        let angle = i as f32 / 90.0 * 2.0 * PI;
        let tilt = 0.42_f32;
        let (ca, sa) = (angle.cos(), angle.sin());
        let pos = Vec3::new(
            ca * 1.55,
            sa * 1.55 * tilt.cos(),
            sa * 1.55 * tilt.sin() - 0.1,
        );
        // Tangent direction in the ring plane, spun into a quaternion
        // that maps local +X onto it: rotation about the ring axis is
        // not expressible as one axis-angle per splat cheaply, so use
        // the z-rotation by the splat angle and let the ring tilt come
        // from the positions — the stretch still reads tangentially in
        // screen space for most of the ring.
        let rot = Quat::from_axis_angle(Vec3::Z, angle + PI / 2.0);
        let shade = 0.75 + 0.25 * (angle * 3.0).sin();
        splats.push(Gaussian::new(
            pos,
            rot,
            Vec3::new(0.22, 0.045, 0.045),
            Color::rgb(0.95 * shade, 0.85 * shade, 0.65 * shade),
            0.8,
        ));
    }

    // A handful of background stars: tiny, dim, far away.
    for _ in 0..40 {
        let pos = Vec3::new(
            (rng.next_f32() - 0.5) * 9.0,
            (rng.next_f32() - 0.5) * 5.0,
            -4.0 - rng.next_f32() * 4.0,
        );
        splats.push(Gaussian::isotropic(
            pos,
            0.03,
            Color::rgb(0.8, 0.85, 1.0),
            0.7,
        ));
    }

    let view = abrash_core::math::Mat4::look_at(Vec3::new(0.0, 0.55, 4.6), Vec3::ZERO, Vec3::Y);
    let camera = SplatCamera::from_fov_y(view, w, h, 50.0_f32.to_radians(), 0.1, 100.0);
    let params = SplatRenderParams {
        background: Color::rgb(0.015, 0.02, 0.045),
        ..SplatRenderParams::default()
    };

    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    let stats = render_gaussians_to_framebuffer(&splats, &camera, &params, &mut fb);
    println!(
        "gaussian_splat_demo: {} splats ({} projected, {} culled), {} fragments evaluated, {} blended, {} skipped (saturated), {} pixels painted",
        stats.splats_input,
        stats.splats_projected,
        stats.splats_culled,
        stats.fragments_evaluated,
        stats.fragments_blended,
        stats.fragments_skipped_saturated,
        stats.pixels_painted,
    );
    fb.export_ppm("gaussian_splat_demo.ppm")
        .expect("writing the demo PPM cannot fail");
    println!("wrote gaussian_splat_demo.ppm ({w}x{h})");
}
