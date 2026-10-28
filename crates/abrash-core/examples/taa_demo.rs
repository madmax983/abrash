//! TAA demo: a bright box slides across a gradient background, two pixels
//! per frame. Motion vectors track the box (and the static background),
//! so the resolve reprojects history onto the same surfaces instead of
//! smearing it; where the box uncovers background, the depth test
//! rejects the box's history.
//!
//! Run with: `cargo run -p abrash-core --example taa_demo`
//! Writes `taa_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_core::taa::{TaaParams, TemporalAccumulator};

const W: usize = 160;
const H: usize = 120;
const BOX_W: usize = 36;
const BOX_H: usize = 28;
const BOX_Y: usize = 46;
const FRAMES: usize = 24;
const STEP: usize = 2; // box speed, px per frame

const fn box_x(frame: usize) -> usize {
    8 + frame * STEP
}

/// Render one synthetic frame: colour, motion, and depth per pixel.
fn render(frame: usize) -> (Vec<Color>, Vec<Vec2>, Vec<f32>) {
    let mut colour = vec![Color::BLACK; W * H];
    let mut motion = vec![Vec2::ZERO; W * H];
    let mut depth = vec![0.9f32; W * H];
    let bx = box_x(frame);
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            // Sky gradient background, static.
            colour[i] = Color::rgb(0.05 + 0.25 * (x as f32 / W as f32), 0.10, 0.30);
            if (bx..bx + BOX_W).contains(&x) && (BOX_Y..BOX_Y + BOX_H).contains(&y) {
                // The box: nearer, warm, moving +STEP px per frame.
                colour[i] = Color::rgb(0.85, 0.45, 0.15);
                motion[i] = Vec2::new(STEP as f32, 0.0);
                depth[i] = 0.4;
            }
        }
    }
    (colour, motion, depth)
}

fn main() {
    let mut acc = TemporalAccumulator::new(W as u32, H as u32).expect("demo dimensions are valid");
    let mut output = vec![Color::BLACK; W * H];
    let mut totals = (0usize, 0usize, 0usize, 0usize);
    for frame in 0..FRAMES {
        let (colour, motion, depth) = render(frame);
        let s = acc.resolve(&colour, &motion, &depth, &TaaParams::default(), &mut output);
        totals.0 += s.history_used;
        totals.1 += s.rejected_depth;
        totals.2 += s.rejected_out_of_bounds;
        totals.3 += s.clamped_pixels;
        if frame == 0 || frame == FRAMES - 1 {
            println!(
                "frame {frame}: used {} | depth-rejected {} | out-of-bounds {} | clamped {}",
                s.history_used, s.rejected_depth, s.rejected_out_of_bounds, s.clamped_pixels
            );
        }
    }
    println!(
        "taa_demo: {FRAMES} frames at {W}x{H} — totals: used {} | depth-rejected {} | out-of-bounds {} | clamped {}",
        totals.0, totals.1, totals.2, totals.3
    );

    let mut fb = Framebuffer::new(W as u32, H as u32).expect("framebuffer dimensions are valid");
    for (dst, src) in fb.as_mut_slice().iter_mut().zip(output.iter()) {
        *dst = src.to_argb_u32();
    }
    fb.export_ppm("taa_demo.ppm")
        .expect("writing the demo PPM cannot fail");
    println!("wrote taa_demo.ppm ({W}x{H})");
}
