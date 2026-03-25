use abrash::experimental::hexagon::apply_hexagon;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Hexagon Filter Demo";

struct HexagonDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl HexagonDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Create a simple gradient background with some dynamic shapes to visualize the honeycomb
        for y in 0..HEIGHT {
            let v = y as f32 / HEIGHT as f32;
            for x in 0..WIDTH {
                let u = x as f32 / WIDTH as f32;

                let r = (u * 255.0) as u32;
                let g = (v * 255.0) as u32;
                let b = ((1.0 - u) * 255.0) as u32;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Add some distinct shapes so we can see the pixelation structure
        for y in 100..300 {
            for x in 100..300 {
                let dx = x as f32 - 200.0;
                let dy = y as f32 - 200.0;
                if dx * dx + dy * dy < 10000.0 {
                    background_fb.set_pixel(x, y, 0xFF_FF_AA_00);
                }
            }
        }

        for y in 200..400 {
            for x in 300..500 {
                let dx = x as f32 - 400.0;
                let dy = y as f32 - 300.0;
                if dx * dx + dy * dy < 8000.0 {
                    background_fb.set_pixel(x, y, 0xFF_00_AA_FF);
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
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

impl WindowApp for HexagonDemoApp {
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
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Restore background
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Animate the honeycomb radius
        let radius = 5.0 + (self.time.sin() * 0.5 + 0.5) * 20.0;

        // Apply Hexagon Filter
        apply_hexagon(&mut self.framebuffer, radius);

        self.present()
    }
}

fn main() -> Result<(), HostError> {
    run_windowed(HexagonDemoApp::new()?)
}
