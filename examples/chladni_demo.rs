#![cfg(feature = "backend-winit")]

use abrash_render::experimental::chladni::{ChladniConfig, apply_chladni};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 600;
const HEIGHT: u32 = 600;

struct ChladniApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    config: ChladniConfig,
    time: f32,
}

impl WindowApp for ChladniApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Chladni Resonance Simulator".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);

        self.config.m = 3.0 + (self.time * 0.5).sin() * 2.0;
        self.config.n = 5.0 + (self.time * 0.7).cos() * 3.0;

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(self.config.bg_color);
        apply_chladni(&mut self.fb, &self.config);

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.fb)?;
        Ok(())
    }
}

fn main() {
    let app = ChladniApp {
        presenter: None,
        fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
        config: ChladniConfig {
            m: 1.0,
            n: 2.0,
            thickness: 0.05,
            color: 0xFF_00FFFF, // Cyan
            bg_color: 0xFF_000000,
        },
        time: 0.0,
    };

    run_windowed(app);
}
