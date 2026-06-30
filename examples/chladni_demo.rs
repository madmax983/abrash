use abrash::framebuffer::Framebuffer;
use abrash::platform::{HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_render::experimental::chladni::generate_chladni_pattern;
use std::time::Instant;

struct ChladniApp {
    fb: Framebuffer,
    time: f32,
    last_frame: Instant,
    presenter: Option<SoftwarePresenter>,
}

impl ChladniApp {
    fn new() -> Self {
        Self {
            fb: Framebuffer::new(800, 600).unwrap(),
            time: 0.0,
            last_frame: Instant::now(),
            presenter: None,
        }
    }
}

impl WindowApp for ChladniApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Chladni Plate Resonance".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.time += dt;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(0xFF_00_00_00);

        let m = 2.0 + (self.time * 0.5).sin();
        let n = 3.0 + (self.time * 0.7).cos();

        generate_chladni_pattern(&mut self.fb, m, n, self.time);

        let fb_ref = &self.fb;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(fb_ref)?;
        Ok(())
    }
}

fn main() {
    run_windowed(ChladniApp::new());
}
