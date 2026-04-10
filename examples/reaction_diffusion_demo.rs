//! Reaction-Diffusion (Gray-Scott) Demo
//!
//! Simulates the Turing patterns formed by two interacting chemicals.
//! Uses a double-buffered grid and 3x3 Laplacian convolutions.

use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::platform::demo::DemoApp;
use abrash::platform::runner::{DemoConfig, run_demo};
use abrash::zbuffer::ZBuffer;
use abrash_core::random::DefaultRng;
use abrash_core::utils::XorShift32;

use abrash::experimental::reaction_diffusion::ReactionDiffusion;

struct ReactionDiffusionDemo {
    sim: ReactionDiffusion,
    color_a: u32,
    color_b: u32,
}

impl DemoApp for ReactionDiffusionDemo {
    fn init(width: u32, height: u32) -> Self {
        // We'll scale down the sim a bit if the window is huge,
        // but for now, 1:1 mapping with the framebuffer size.
        let sim_w = (width / 2) as usize; // run at half-res for speed and thicker lines
        let sim_h = (height / 2) as usize;

        let mut sim = ReactionDiffusion::new(sim_w, sim_h);

        // Seed some initial spots
        let mut rng = DefaultRng::new(42);

        sim.seed(sim_w / 2, sim_h / 2, 10);
        sim.seed(sim_w / 3, sim_h / 3, 5);
        sim.seed(sim_w * 2 / 3, sim_h * 2 / 3, 8);
        sim.seed_random(&mut rng, sim_w / 4);

        Self {
            sim,
            color_a: 0xFF_110022, // Deep purple/black
            color_b: 0xFF_00FFAA, // Bright cyan/green
        }
    }

    fn update(&mut self, _dt: f32) {
        // We can run multiple steps per frame to speed up the visual evolution
        for _ in 0..10 {
            self.sim.step();
        }
    }

    fn draw(&mut self, fb: &mut Framebuffer, _zb: &mut ZBuffer) {
        // Clear background
        fb.clear(self.color_a);

        // We run the sim at half-res, but we'll manually render it to fill the fb.
        let w = self.sim.width.min(fb.width() as usize / 2);
        let h = self.sim.height.min(fb.height() as usize / 2);

        // To avoid having to implement a scaling render, let's just make a temp fb,
        // render the sim to it, and nearest-neighbor scale it up.
        let mut temp_fb = Framebuffer::new(w as u32, h as u32).unwrap();
        self.sim.render(&mut temp_fb, self.color_a, self.color_b);

        let fb_pixels = fb.as_mut_slice();
        let fb_w = fb.width() as usize;

        for y in 0..h {
            for x in 0..w {
                let pixel = temp_fb.get_pixel(x as u32, y as u32).unwrap_or(0);

                // 2x Nearest Neighbor Scale
                let dest_y = y * 2;
                let dest_x = x * 2;

                if dest_y + 1 < fb.height() as usize && dest_x + 1 < fb_w {
                    fb_pixels[dest_y * fb_w + dest_x] = pixel;
                    fb_pixels[dest_y * fb_w + dest_x + 1] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x + 1] = pixel;
                }
            }
        }
    }
}

fn main() {
    let config = DemoConfig {
        title: "🌟 Nova: Reaction-Diffusion (Gray-Scott)",
        width: 800,
        height: 600,
        scale: 1,
        fps_limit: 60,
    };
    run_demo::<ReactionDiffusionDemo>(config);
}
