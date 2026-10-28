//! Radiance Cascades demo: a small room with a dividing wall, a
//! doorway gap, two lamps, and pillars, solved with the 2D cascade
//! solver and written out as a PPM image of the per-pixel radiance.
//!
//! Run with: `cargo run -p abrash-core --example radiance_cascades_demo`
//! Writes `radiance_cascades_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::radiance_cascades::{RadianceCascades, RcCell, RcConfig, RcScene};

fn main() {
    let (w, h) = (64, 36);
    let mut scene = RcScene::empty(w, h).expect("scene dimensions are valid");

    // Dividing wall at x = 30, with a three-cell doorway gap.
    for y in 0..h {
        if !(17..20).contains(&y) {
            scene.set_cell(30, y, RcCell::Occluder).expect("wall cell");
        }
    }
    // A shelf on the left side.
    for x in 6..26 {
        scene.set_cell(x, 10, RcCell::Occluder).expect("shelf cell");
    }
    // Two free-standing pillars.
    for (px, py) in [(12, 24), (46, 8)] {
        for dy in 0..2 {
            for dx in 0..2 {
                scene
                    .set_cell(px + dx, py + dy, RcCell::Occluder)
                    .expect("pillar cell");
            }
        }
    }
    // Warm lamp on the left (3x2), cool lamp on the right (2x2),
    // and one small green light by the doorway.
    for dy in 0..2 {
        for dx in 0..3 {
            scene
                .set_cell(8 + dx, 16 + dy, RcCell::Emitter([2.2, 1.2, 0.4]))
                .expect("warm lamp cell");
        }
        for dx in 0..2 {
            scene
                .set_cell(52 + dx, 26 + dy, RcCell::Emitter([0.3, 0.8, 2.0]))
                .expect("cool lamp cell");
        }
    }
    scene
        .set_cell(44, 18, RcCell::Emitter([0.2, 1.5, 0.3]))
        .expect("green lamp cell");

    let config = RcConfig {
        environment: [0.03, 0.03, 0.04],
        ..RcConfig::default()
    };
    let solver = RadianceCascades::new(scene, config).expect("configuration is valid");

    for level in solver.levels() {
        println!(
            "cascade {}: spacing {}, probes {}x{}, {} directions, interval [{:.1}, {:.1})",
            level.index,
            level.spacing,
            level.probe_width,
            level.probe_height,
            level.direction_count,
            level.t_start,
            level.t_end
        );
    }

    // Radiance is HDR (lamps emit above 1.0); clamp into the
    // framebuffer range for the image.
    let mut fb = Framebuffer::new(w as u32, h as u32).expect("framebuffer dimensions are valid");
    let mut sum = [0.0f32; 3];
    let mut peak = 0.0f32;
    let mut lit = 0usize;
    for y in 0..h {
        for x in 0..w {
            let px = solver.output_pixel(x, y).unwrap_or([0.0; 3]);
            for channel in 0..3 {
                sum[channel] += px[channel];
            }
            peak = peak.max(px[0]).max(px[1]).max(px[2]);
            if px[0] + px[1] + px[2] > 0.05 {
                lit += 1;
            }
            let color = Color::rgb(
                px[0].clamp(0.0, 1.0),
                px[1].clamp(0.0, 1.0),
                px[2].clamp(0.0, 1.0),
            );
            fb.set_pixel(x as i32, y as i32, color.to_argb_u32());
        }
    }
    let count = (w * h) as f32;
    fb.export_ppm("radiance_cascades_demo.ppm")
        .expect("write PPM");
    println!(
        "radiance_cascades_demo: {w}x{h}, {} cascades, mean radiance [{:.3}, {:.3}, {:.3}], peak {peak:.2}, {lit} lit pixels, wrote radiance_cascades_demo.ppm",
        solver.level_count(),
        sum[0] / count,
        sum[1] / count,
        sum[2] / count
    );
}
