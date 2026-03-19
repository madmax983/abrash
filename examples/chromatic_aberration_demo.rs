use abrash::experimental::chromatic_aberration::{
    ChromaticAberrationConfig, apply_chromatic_aberration,
};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Chromatic Aberration Demo";

struct ChromaticAberrationDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    shift_amount: f32,
}

impl ChromaticAberrationDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            shift_amount: 0.0,
        })
    }
}

impl WindowApp for ChromaticAberrationDemo {
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.shift_amount += ctx.dt_seconds * 5.0;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;
        let pixels = self.framebuffer.as_mut_slice();

        for y in 0..height {
            for x in 0..width {
                let is_grid_line = (x % 50 < 2) || (y % 50 < 2);
                let is_circle = ((x as i32 - width as i32 / 2).pow(2)
                    + (y as i32 - height as i32 / 2).pow(2))
                    < 150_i32.pow(2);

                pixels[y * width + x] = if is_grid_line {
                    0xFF_AA_AA_AA
                } else if is_circle {
                    0xFF_FF_FF_FF
                } else {
                    0xFF_22_22_22
                };
            }
        }

        let shift = (self.shift_amount.sin() * 10.0) as i32;
        let config = ChromaticAberrationConfig {
            red_shift: (shift, 0),
            green_shift: (0, 0),
            blue_shift: (-shift, 0),
        };

        apply_chromatic_aberration(&mut self.framebuffer, &config);
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    run_windowed(ChromaticAberrationDemo::new()?)
}
