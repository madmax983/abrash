//! Physarum (Slime Mold) Simulation Demo
//!
//! Simulates a transport network forming based on Physarum Polycephalum.
//! Agents drop a trail map and sense it to steer towards high concentrations.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use abrash_render::experimental::physarum::PhysarumSimulation;

struct PhysarumDemo {
    sim: PhysarumSimulation,
    color_bg: u32,
    color_trail: u32,
    presenter: Option<SoftwarePresenter>,
}

impl PhysarumDemo {
    fn new() -> Self {
        let width = 800;
        let height = 600;

        let sim = PhysarumSimulation::new(width, height, 10_000);

        Self {
            sim,
            color_bg: 0xFF_000000,
            color_trail: 0xFF_00FFCC,
            presenter: None,
        }
    }
}

impl WindowApp for PhysarumDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Physarum (Slime Mold)".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if self.presenter.is_none() {
            self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        }

        // Run multiple steps for speed
        for _ in 0..2 {
            self.sim.step();
        }

        let presenter = self.presenter.as_mut().unwrap();

        let mut fb_owned = Framebuffer::new(800, 600).unwrap();
        fb_owned.clear(self.color_bg);

        self.sim.render(&mut fb_owned, self.color_trail);

        presenter.present(&fb_owned)?;
        ctx.window.request_redraw();

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_windowed(PhysarumDemo::new());
    Ok(())
}
