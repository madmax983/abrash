use abrash::experimental::pointillism::apply_pointillism;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::convert::Infallible;

struct PointillismDemo {
    fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
}

impl WindowApp for PointillismDemo {
    type Error = Infallible;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: 800,
            height: 600,
            title: "Pointillism Demo".to_string(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let w = self.fb.width() as i32;
        let h = self.fb.height() as i32;
        for y in 0..h {
            for x in 0..w {
                let r = (x * 255 / w) as u32;
                let g = (y * 255 / h) as u32;
                let color = 0xFF00_0000 | (r << 16) | (g << 8) | 128;
                self.fb.set_pixel(x, y, color);
            }
        }
        let cx = w / 2;
        let cy = h / 2;
        let radius = 150;
        for y in -radius..radius {
            for x in -radius..radius {
                if x * x + y * y <= radius * radius {
                    self.fb.set_pixel(cx + x, cy + y, 0xFFFF_0000);
                }
            }
        }
        apply_pointillism(&mut self.fb, 12);

        self.presenter = Some(SoftwarePresenter::new(ctx.window).unwrap());
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb).unwrap();
        }
        Ok(())
    }
}

fn main() {
    let app = PointillismDemo {
        fb: Framebuffer::new(800, 600).unwrap(),
        presenter: None,
    };
    run_windowed(app);
}
