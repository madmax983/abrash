#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::oil_paint::apply_oil_paint;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Oil Paint Filter Demo";

struct OilPaintDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
}

impl OilPaintDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            time: 0.0,
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

impl WindowApp for OilPaintDemoApp {
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0) * 1.0;

        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;

                let r = (f32::sin(u * 10.0 + self.time) * 127.5 + 127.5) as u32;
                let g = (f32::cos(v * 10.0 + self.time) * 127.5 + 127.5) as u32;
                let b = (f32::sin((u + v) * 10.0 - self.time) * 127.5 + 127.5) as u32;

                self.framebuffer.as_mut_slice()[(y * width + x) as usize] =
                    0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        apply_oil_paint(&mut self.framebuffer, 4, 20);
        self.present()
    }
}

fn main() -> Result<(), HostError> {
    let app = OilPaintDemoApp::new()?;
    run_windowed(app);
    Ok(())
}
