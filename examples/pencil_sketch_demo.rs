use abrash::experimental::pencil_sketch::{PencilSketchConfig, apply_pencil_sketch};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Pencil Sketch Filter Demo";

struct PencilSketchDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    config: PencilSketchConfig,
    time: f32,
}

impl PencilSketchDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw some procedural shapes that will look good with a pencil sketch filter
        let w = WIDTH as i32;
        let h = HEIGHT as i32;

        background_fb.clear(0xFFF0F0F0); // Off-white paper background

        // Checkerboard
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if (x / 64 + y / 64) % 2 == 0 {
                    background_fb.set_pixel(x as i32, y as i32, 0xFF888888);
                }
            }
        }

        // Draw a few overlapping circles
        for i in 0..5 {
            let cx = w / 2 + (i * 40 - 80);
            let cy = h / 2;
            let radius = 60 + i * 10;

            for y in (cy - radius)..=(cy + radius) {
                for x in (cx - radius)..=(cx + radius) {
                    if x >= 0 && x < w && y >= 0 && y < h {
                        let dx = x - cx;
                        let dy = y - cy;
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq < radius * radius {
                            // Simple gradient inside the circle
                            let dist = (dist_sq as f32).sqrt();
                            let luma = 255 - ((dist / radius as f32) * 200.0) as u32;
                            // Add some color variance to see how sketch handles it
                            let r = (luma * (100 + i as u32 * 20)) / 255;
                            let g = (luma * (200 - i as u32 * 20)) / 255;
                            let b = (luma * 150) / 255;
                            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                            background_fb.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            config: PencilSketchConfig {
                intensity: 1.0,
                blur_radius: 5,
            },
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

impl WindowApp for PencilSketchDemoApp {
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
        // Animate intensity to show before/after
        self.config.intensity = (self.time.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());
        apply_pencil_sketch(&mut self.framebuffer, &self.config);
        self.present()
    }
}

fn main() -> Result<(), HostError> {
    run_windowed(PencilSketchDemoApp::new()?)
}
