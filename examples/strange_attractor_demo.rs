#![cfg(feature = "backend-winit")]
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::strange_attractor::render_clifford_attractor;

struct AttractorApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

impl AttractorApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            framebuffer: Framebuffer::new(800, 600).map_err(|e| HostError::App(e.to_string()))?,
            presenter: None,
            time: 0.0,
        })
    }
}

impl WindowApp for AttractorApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Clifford Attractor".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_00_00_00);
        let a = 1.5 + (self.time * 0.1).sin();
        let b = -1.8 + (self.time * 0.15).cos();
        let c = 1.6 + (self.time * 0.12).sin();
        let d = 2.0 + (self.time * 0.11).cos();
        render_clifford_attractor(&mut self.framebuffer, a, b, c, d, 2_000_000);

        let fb = &self.framebuffer;
        let presenter = self.presenter.as_mut().unwrap();
        presenter.present(fb)?;
        Ok(())
    }
}

fn main() {
    run_windowed(AttractorApp::new().unwrap());
}
