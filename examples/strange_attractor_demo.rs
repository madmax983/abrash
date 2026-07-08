//! Strange Attractor (Peter de Jong) Demo
//!
//! Generates a Peter de Jong strange attractor.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::color::Color;
use abrash_render::experimental::strange_attractor::{render_strange_attractor, AttractorConfig};

struct StrangeAttractorDemo {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    config: AttractorConfig,
    host_config: WindowHostConfig,
}

impl StrangeAttractorDemo {
    fn new(width: u32, height: u32) -> Self {
        Self {
            presenter: None,
            fb: Framebuffer::new(width, height).unwrap(),
            config: AttractorConfig {
                a: -2.24,
                b: 0.43,
                c: -0.65,
                d: -2.43,
                iterations: 1_000_000,
                base_color: Color::new(0.3, 0.6, 1.0, 1.0),
            },
            host_config: WindowHostConfig {
                title: "Strange Attractor Demo".to_string(),
                width,
                height,
                vsync: true,
            },
        }
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.fb;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for StrangeAttractorDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        self.host_config.clone()
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Slowly animate parameters
        self.config.a += 0.001;
        self.config.b -= 0.001;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(0xFF00_0000);
        render_strange_attractor(&mut self.fb, &self.config);
        self.present()
    }
}

fn main() {
    println!("Starting Strange Attractor Demo...");
    let app = StrangeAttractorDemo::new(800, 600);
    run_windowed(app);
}
