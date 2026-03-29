use abrash::experimental::radar::{RadarConfig, apply_radar};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Radar Filter Demo";

struct RadarDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    config: RadarConfig,
}

impl RadarDemoApp {
    fn new() -> Result<Self, HostError> {
        let framebuffer =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;
        Ok(Self {
            presenter: None,
            framebuffer,
            config: RadarConfig::default(),
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for RadarDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: WIDTH,
            height: HEIGHT,
            title: TITLE.to_string(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.config.angle += ctx.dt_seconds.max(0.0) * 2.0;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_222222);

        // Draw some "blips" to test luminance overlay
        for y in 200..220 {
            for x in 300..320 {
                self.framebuffer.set_pixel(x, y, 0xFF_FFFFFF);
            }
        }

        apply_radar(&mut self.framebuffer, &self.config);
        self.present()
    }
}

fn main() -> Result<(), HostError> {
    run_windowed(RadarDemoApp::new()?)
}
