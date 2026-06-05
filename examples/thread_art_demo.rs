use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::thread_art::{ThreadArt, ThreadArtConfig};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Thread Art Generator Demo";

struct DemoApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    art: ThreadArt,
}

impl DemoApp {
    fn new() -> Result<Self, HostError> {
        let mut fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a procedural Yin-Yang-like or circular target shape for the string art to approximate
        fb.clear(0xFF_FFFFFF);
        let cx = WIDTH as i32 / 2;
        let cy = HEIGHT as i32 / 2;
        let r = 250;

        for y in 0..HEIGHT as i32 {
            for x in 0..WIDTH as i32 {
                let dx = x - cx;
                let dy = y - cy;

                // Outer circle
                if dx * dx + dy * dy < r * r {
                    if dx < 0 {
                        fb.set_pixel(x, y, 0xFF_000000);
                    }

                    // Inner circles
                    let d_top = dx * dx + (dy - r / 2) * (dy - r / 2);
                    let d_bot = dx * dx + (dy + r / 2) * (dy + r / 2);

                    if d_top < (r / 2) * (r / 2) {
                        fb.set_pixel(x, y, 0xFF_000000);
                    }
                    if d_bot < (r / 2) * (r / 2) {
                        fb.set_pixel(x, y, 0xFF_FFFFFF);
                    }

                    // Small dots
                    if d_top < (r / 8) * (r / 8) {
                        fb.set_pixel(x, y, 0xFF_FFFFFF);
                    }
                    if d_bot < (r / 8) * (r / 8) {
                        fb.set_pixel(x, y, 0xFF_000000);
                    }
                }
            }
        }

        Ok(Self {
            presenter: None,
            fb,
            art: ThreadArt::new(ThreadArtConfig {
                num_pins: 256,
                max_lines: 3000,
                lines_per_frame: 10,
                thread_color: 0xFF_000000,
                bg_color: 0xFF_FFFFFF,
                line_weight: 0.15,
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
        // Since ThreadArt reads the framebuffer initially, resizing breaks the target image
        // For this demo, we'll just ignore resize and keep the fixed buffer or panic
        // Or we could redraw the yin-yang here. Let's redraw.

        self.fb =
            Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        self.fb.clear(0xFF_FFFFFF);
        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let r = (width.min(height) as i32) / 2 - 20;

        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let dx = x - cx;
                let dy = y - cy;

                if dx * dx + dy * dy < r * r {
                    if dx < 0 {
                        self.fb.set_pixel(x, y, 0xFF_000000);
                    }

                    let d_top = dx * dx + (dy - r / 2) * (dy - r / 2);
                    let d_bot = dx * dx + (dy + r / 2) * (dy + r / 2);

                    if d_top < (r / 2) * (r / 2) {
                        self.fb.set_pixel(x, y, 0xFF_000000);
                    }
                    if d_bot < (r / 2) * (r / 2) {
                        self.fb.set_pixel(x, y, 0xFF_FFFFFF);
                    }

                    if d_top < (r / 8) * (r / 8) {
                        self.fb.set_pixel(x, y, 0xFF_FFFFFF);
                    }
                    if d_bot < (r / 8) * (r / 8) {
                        self.fb.set_pixel(x, y, 0xFF_000000);
                    }
                }
            }
        }

        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.art.update_and_render(&mut self.fb);

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
