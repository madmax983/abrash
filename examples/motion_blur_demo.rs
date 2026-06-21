use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::experimental::motion_blur::{MotionBlurConfig, MotionBlurEffect};

struct MotionBlurDemo {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    effect: MotionBlurEffect,
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
}

impl WindowApp for MotionBlurDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Motion Blur Demo".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.x += self.vx;
        self.y += self.vy;

        if self.x < 50.0 || self.x > 750.0 {
            self.vx *= -1.0;
        }
        if self.y < 50.0 || self.y > 550.0 {
            self.vy *= -1.0;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear background to black
        self.framebuffer.clear(0xFF00_0000);

        // Draw a moving square
        let rx = self.x as i32 - 25;
        let ry = self.y as i32 - 25;
        for i in 0..50 {
            for j in 0..50 {
                if rx + i >= 0 && ry + j >= 0 && rx + i < 800 && ry + j < 600 {
                   self.framebuffer.set_pixel(rx + i, ry + j, 0xFFFF_0000);
                }
            }
        }

        // Apply motion blur
        self.effect.apply(&mut self.framebuffer);

        if let Some(presenter) = &mut self.presenter {
            let _ = presenter.present(&self.framebuffer);
        }

        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), HostError> {
    let app = MotionBlurDemo {
        x: 400.0,
        y: 300.0,
        vx: 15.0,
        vy: 10.0,
        effect: MotionBlurEffect::new(MotionBlurConfig { blend_factor: 0.8 }),
        framebuffer: Framebuffer::new(800, 600).unwrap(),
        presenter: None,
    };
    run_windowed(app);
    Ok(())
}
