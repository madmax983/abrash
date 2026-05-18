use abrash::framebuffer::Framebuffer;
use abrash::platform::{HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_render::experimental::metaballs::{Metaballs, MetaballsConfig};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Metaballs / Implicit Surfaces Demo";

struct DemoApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    metaballs: Metaballs,
}

impl DemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            fb: Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?,
            metaballs: Metaballs::new(MetaballsConfig {
                num_balls: 10,
                threshold: 1.0,
                blob_color: 0xFF_FF00FF, // Magenta blobs
                bg_color: 0xFF_111111,
                speed: 5.0,
                max_size: 40.0,
            }),
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
        self.fb = Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // State updates are handled inside update_and_render for this demo
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.metaballs.update_and_render(&mut self.fb);

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
