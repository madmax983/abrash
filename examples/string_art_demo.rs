use abrash::framebuffer::Framebuffer;
use abrash::experimental::string_art::{apply_string_art, StringArtConfig};
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

struct StringArtDemoApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    config: StringArtConfig,
    lines_drawn: usize,
    max_lines: usize,
}

impl WindowApp for StringArtDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "String Art Demo".to_string(),
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
        if width > 0 && height > 0 && let Ok(fb) = Framebuffer::new(width, height) {
            self.framebuffer = fb;
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Increment lines drawn over time
        if self.lines_drawn < self.max_lines {
            self.lines_drawn = (self.lines_drawn + 10).min(self.max_lines);
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Base image: We will draw a simple shape for the string art to recreate.
        self.framebuffer.clear(0xFF_FF_FF_FF);

        // Draw a dark circle in the middle as the "target" shape.
        let center_x = self.framebuffer.width() as i32 / 2;
        let center_y = self.framebuffer.height() as i32 / 2;
        let radius = self.framebuffer.width() as i32 / 3;

        abrash::rasterizer::fill_circle(&mut self.framebuffer, center_x, center_y, radius, 0xFF_00_00_00);

        // Draw some "eyes"
        abrash::rasterizer::fill_circle(&mut self.framebuffer, center_x - radius / 3, center_y - radius / 3, radius / 5, 0xFF_FF_FF_FF);
        abrash::rasterizer::fill_circle(&mut self.framebuffer, center_x + radius / 3, center_y - radius / 3, radius / 5, 0xFF_FF_FF_FF);

        // Draw a "mouth"
        abrash::rasterizer::fill_rect(&mut self.framebuffer, center_x - radius / 2, center_y + radius / 3, radius as u32, (radius / 5) as u32, 0xFF_FF_FF_FF);

        // Apply String Art
        self.config.num_lines = self.lines_drawn;
        apply_string_art(&mut self.framebuffer, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

fn main() {
    let fb = Framebuffer::new(800, 800).unwrap();
    let app = StringArtDemoApp {
        framebuffer: fb,
        presenter: None,
        config: StringArtConfig {
            num_pins: 256,
            num_lines: 0,
            thread_color: 0xFF_00_00_00,
            background_color: 0xFF_FF_FF_FF,
            thread_alpha: 0.15,
            radius: 0.95,
        },
        lines_drawn: 0,
        max_lines: 2000,
    };
    run_windowed(app);
}
