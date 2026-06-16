use abrash::experimental::vectorscope::{VectorscopeConfig, apply_vectorscope};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

struct VectorscopeDemo {
    fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

impl VectorscopeDemo {
    fn new(width: u32, height: u32) -> Self {
        Self {
            fb: Framebuffer::new(width, height).unwrap(),
            presenter: None,
            time: 0.0,
        }
    }
}

impl WindowApp for VectorscopeDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Vectorscope Demo".to_string(),
            width: self.fb.width(),
            height: self.fb.height(),
            ..Default::default()
        }
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if self.presenter.is_none() {
            self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        }

        let width = self.fb.width();
        let height = self.fb.height();

        // Draw a colorful animating pattern
        let time_val = self.time;
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;

                let r = ((u * 10.0 + time_val).sin() * 0.5 + 0.5) * 255.0;
                let g = ((v * 10.0 - time_val).cos() * 0.5 + 0.5) * 255.0;
                let b = (((u + v) * 5.0 + time_val * 2.0).sin() * 0.5 + 0.5) * 255.0;

                let color = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                self.fb.set_pixel(x as i32, y as i32, color);
            }
        }

        let config = VectorscopeConfig {
            center_x: 150,
            center_y: 150,
            radius: 120,
            intensity: 0.05,
            draw_background: true,
            draw_grid: true,
        };

        apply_vectorscope(&mut self.fb, &config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }
}

fn main() {
    let app = VectorscopeDemo::new(800, 600);
    run_windowed(app);
}
