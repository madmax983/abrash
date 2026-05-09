//! Synthwave Grid Post-Processing Filter Demo
//!
//! Visualizes a retro 3D perspective grid moving towards the camera.

use abrash::experimental::synthwave_grid::{SynthwaveGridConfig, apply_synthwave_grid};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Abrash - Synthwave Grid";

struct SynthwaveGridDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
    config: SynthwaveGridConfig,
}

impl SynthwaveGridDemo {
    fn new() -> Result<Self, HostError> {
        let config = SynthwaveGridConfig {
            horizon: HEIGHT as f32 * 0.4,
            grid_color: 0xFF_FF00FF,
            sky_color: 0xFF_090022,
            ground_color: 0xFF_000000,
            grid_spacing: 15.0,
            line_thickness: 1.5,
            speed: 40.0,
            time: 0.0,
            fov: 150.0,
            camera_height: 30.0,
            fog_distance: 600.0,
        };

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            time: 0.0,
            config,
        })
    }
}

impl WindowApp for SynthwaveGridDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        self.config.time = self.time;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        apply_synthwave_grid(&mut self.framebuffer, &self.config);

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;
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
        self.config.horizon = height as f32 * 0.4;
        Ok(())
    }
}

fn main() {
    run_windowed(SynthwaveGridDemo::new().unwrap());
}
