//! Reaction-Diffusion (Gray-Scott) Demo
//!
//! Simulates the Turing patterns formed by two interacting chemicals.
//! Uses a double-buffered grid and 3x3 Laplacian convolutions.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::utils::XorShift32;

use abrash::experimental::reaction_diffusion::ReactionDiffusion;

struct ReactionDiffusionDemo {
    sim: ReactionDiffusion,
    color_a: u32,
    color_b: u32,
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
}

impl ReactionDiffusionDemo {
    fn new(width: u32, height: u32) -> Self {
        let sim_w = (width / 2) as usize; // run at half-res for speed and thicker lines
        let sim_h = (height / 2) as usize;

        let mut sim = ReactionDiffusion::new(sim_w, sim_h);

        let mut rng = XorShift32::new(42);

        sim.seed(sim_w / 2, sim_h / 2, 10);
        sim.seed(sim_w / 3, sim_h / 3, 5);
        sim.seed(sim_w * 2 / 3, sim_h * 2 / 3, 8);
        sim.seed_random(&mut rng, sim_w / 4);

        Self {
            sim,
            color_a: 0xFF_110022, // Deep purple/black
            color_b: 0xFF_00FFAA, // Bright cyan/green
            presenter: None,
            framebuffer: Framebuffer::new(width, height).unwrap(),
        }
    }
}

impl WindowApp for ReactionDiffusionDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Reaction-Diffusion (Gray-Scott)".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        for _ in 0..10 {
            self.sim.step();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(self.color_a);

        let fb_w = self.framebuffer.width() as usize;
        let fb_h = self.framebuffer.height() as usize;

        let w = self.sim.width.min(fb_w / 2);
        let h = self.sim.height.min(fb_h / 2);

        let mut temp_fb = Framebuffer::new(w as u32, h as u32).unwrap();
        self.sim.render(&mut temp_fb, self.color_a, self.color_b);

        let fb_pixels = self.framebuffer.as_mut_slice();

        for y in 0..h {
            for x in 0..w {
                let pixel = temp_fb.get_pixel(x as i32, y as i32).unwrap_or(0);

                let dest_y = y * 2;
                let dest_x = x * 2;

                if dest_y + 1 < fb_h && dest_x + 1 < fb_w {
                    fb_pixels[dest_y * fb_w + dest_x] = pixel;
                    fb_pixels[dest_y * fb_w + dest_x + 1] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x + 1] = pixel;
                }
            }
        }

        let fb_copy = &self.framebuffer;
        if let Some(presenter) = &mut self.presenter {
            presenter.present(fb_copy)?;
        }
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() {
    run_windowed(ReactionDiffusionDemo::new(800, 600));
}
