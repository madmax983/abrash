use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::strange_attractor::{AttractorConfig, StrangeAttractor};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Strange Attractor Demo";

struct DemoApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    attractor: StrangeAttractor,
    time: f64,
}

impl DemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            fb: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            attractor: StrangeAttractor::new(AttractorConfig {
                iterations: 500_000,
                scale: 0.15,
                ..Default::default()
            }),
            time: 0.0,
        })
    }
}

impl WindowApp for DemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: self.fb.width(),
            height: self.fb.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.fb =
            Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += 0.01;
        // Slowly mutate parameters to animate the chaotic system over time
        self.attractor.config.a = -1.4 + (self.time * 0.5).sin() * 0.2;
        self.attractor.config.b = 1.6 + (self.time * 0.7).cos() * 0.2;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear background
        self.fb.clear(0xFF_050511);

        // Render attractor
        self.attractor.render(&mut self.fb);

        let fb = &self.fb;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(fb)?;
        Ok(())
    }
}

fn main() {
    run_windowed(DemoApp::new().unwrap());
}
