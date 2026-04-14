use abrash::experimental::tunnel::apply_tunnel;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::time::Instant;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "🌟 Nova: Tunnel Effect Demo";

struct TunnelDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    start_time: Instant,
}

impl TunnelDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            start_time: Instant::now(),
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

impl WindowApp for TunnelDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
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
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // 1. Clear frame
        self.framebuffer.clear(0xFF000000);

        // 2. Apply tunnel effect
        apply_tunnel(&mut self.framebuffer, elapsed);

        self.present()
    }
}

fn main() {
    run_windowed(TunnelDemoApp::new().unwrap())
}
