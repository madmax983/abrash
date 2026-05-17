#![cfg(feature = "backend-winit")]

use abrash::experimental::fractal::render_julia;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::time::Instant;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Julia Set Demo";

struct JuliaDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    start_time: Instant,
}

impl JuliaDemoApp {
    pub fn new() -> Self {
        Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            start_time: Instant::now(),
        }
    }
}

impl JuliaDemoApp {
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

impl WindowApp for JuliaDemoApp {
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
        self.start_time = Instant::now();
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let c_re = (elapsed * 0.5).sin() * 0.7885;
        let c_im = (elapsed * 0.5).cos() * 0.7885;

        // Render the Julia set
        render_julia(&mut self.framebuffer, c_re, c_im, 0.0, 0.0, 1.5, 50);

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.present()
    }
}

fn main() {
    println!("Starting Julia Set Demo...");
    run_windowed(JuliaDemoApp::new());
}
