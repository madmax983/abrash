use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::string_art::{StringArtConfig, apply_string_art};
use std::time::Instant;

struct StringArtApp {
    fb: Framebuffer,
    generated: bool,
    config: WindowHostConfig,
    presenter: Option<SoftwarePresenter>,
}

impl WindowApp for StringArtApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        self.config.clone()
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), HostError> {
        if !self.generated {
            // Create a target image (a gradient circle)
            self.fb.clear(0xFF_FF_FF_FF);
            let cx = 400.0;
            let cy = 400.0;
            for y in 0..800 {
                for x in 0..800 {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dist = dx.hypot(dy);
                    if dist < 300.0 {
                        let intensity = (dist / 300.0).clamp(0.0, 1.0);
                        let luma = (intensity * 255.0) as u32;
                        self.fb.set_pixel(
                            x,
                            y,
                            0xFF00_0000 | (luma << 16) | (luma << 8) | luma,
                        );
                    }
                }
            }

            println!("Generating string art... This may take a moment.");

            let start = Instant::now();
            let config = StringArtConfig {
                num_pins: 256,
                num_lines: 4000,
                ..Default::default()
            };

            apply_string_art(&mut self.fb, &config);
            println!("Finished in {:?}", start.elapsed());
            self.generated = true;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), HostError> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }
}

fn main() {
    let config = WindowHostConfig {
        width: 800,
        height: 800,
        title: "String Art Effect".to_string(),
        ..Default::default()
    };

    let app = StringArtApp {
        fb: Framebuffer::new(800, 800).unwrap(),
        generated: false,
        config,
        presenter: None,
    };

    run_windowed(app);
}
