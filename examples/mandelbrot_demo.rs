//! Mandelbrot Demo
//!
//! Renders the Mandelbrot set and allows panning and zooming.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use abrash_render::experimental::mandelbrot::{MandelbrotConfig, render_mandelbrot};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

struct MandelbrotDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    #[cfg(feature = "nova")]
    config: MandelbrotConfig,
}

impl MandelbrotDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            #[cfg(feature = "nova")]
            config: MandelbrotConfig::default(),
        })
    }
}

impl WindowApp for MandelbrotDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Mandelbrot Demo".to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
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
        self.framebuffer =
            Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Here we would handle input for zooming/panning
        // To keep it simple for now, we'll auto-zoom
        #[cfg(feature = "nova")]
        {
            self.config.zoom *= 1.01;
            // self.config.center_x += 0.001 / self.config.zoom;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        #[cfg(feature = "nova")]
        render_mandelbrot(&mut self.framebuffer, &self.config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

// Fallback for when "nova" feature is not enabled
#[cfg(not(feature = "nova"))]
fn main() {
    eprintln!("This example requires the 'nova' feature to run.");
    std::process::exit(1);
}

#[cfg(feature = "nova")]
fn main() {
    println!("🌟 Nova: Mandelbrot Demo");
    run_windowed(MandelbrotDemo::new().unwrap());
}
