//! Reaction-Diffusion (Gray-Scott) Demo
//!
//! Simulates the Turing patterns formed by two interacting chemicals.
//! Uses a double-buffered grid and 3x3 Laplacian convolutions.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use abrash::experimental::reaction_diffusion::ReactionDiffusion;
use std::fmt;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        Self(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

struct ReactionDiffusionDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    sim: ReactionDiffusion,
    color_a: u32,
    color_b: u32,
}

impl ReactionDiffusionDemo {
    fn new() -> Result<Self, AppError> {
        // We'll scale down the sim a bit if the window is huge,
        // but for now, 1:1 mapping with the framebuffer size.
        let sim_w = (WIDTH / 2) as usize; // run at half-res for speed and thicker lines
        let sim_h = (HEIGHT / 2) as usize;

        let mut sim = ReactionDiffusion::new(sim_w, sim_h);

        // Seed some initial spots
        let mut rng = abrash_core::utils::XorShift32::new(42);

        sim.seed(sim_w / 2, sim_h / 2, 10);
        sim.seed(sim_w / 3, sim_h / 3, 5);
        sim.seed(sim_w * 2 / 3, sim_h * 2 / 3, 8);
        sim.seed_random(&mut rng, sim_w / 4);

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            sim,
            color_a: 0xFF_110022, // Deep purple/black
            color_b: 0xFF_00FFAA, // Bright cyan/green
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| std::io::Error::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for ReactionDiffusionDemo {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Reaction-Diffusion (Gray-Scott)".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // We can run multiple steps per frame to speed up the visual evolution
        for _ in 0..10 {
            self.sim.step();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let fb = &mut self.framebuffer;
        // Clear background
        fb.clear(self.color_a);

        // We run the sim at half-res, but we'll manually render it to fill the fb.
        let w = self.sim.width.min(fb.width() as usize / 2);
        let h = self.sim.height.min(fb.height() as usize / 2);

        // To avoid having to implement a scaling render, let's just make a temp fb,
        // render the sim to it, and nearest-neighbor scale it up.
        let mut temp_fb = Framebuffer::new(w as u32, h as u32).unwrap();
        self.sim.render(&mut temp_fb, self.color_a, self.color_b);

        let fb_w = fb.width() as usize;
        let fb_h = fb.height() as usize;
        let fb_pixels = fb.as_mut_slice();

        for y in 0..h {
            for x in 0..w {
                let pixel = temp_fb.get_pixel(x as i32, y as i32).unwrap_or(0);

                // 2x Nearest Neighbor Scale
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

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    run_windowed(ReactionDiffusionDemo::new().unwrap());
    Ok(())
}
