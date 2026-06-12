use abrash::platform::winit::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sandpile::{SandpileConfig, apply_sandpile};
use std::fmt;
use winit::event::WindowEvent;
use winit::keyboard::{Key, NamedKey};

#[derive(Debug)]
struct SandpileError(String);

impl fmt::Display for SandpileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SandpileError {}

struct SandpileDemo {
    config: SandpileConfig,
    frames: usize,
    presenter: Option<SoftwarePresenter>,
    fb: Option<Framebuffer>,
}

impl SandpileDemo {
    fn new() -> Self {
        Self {
            config: SandpileConfig {
                drop_x: 0,
                drop_y: 0,
                grains_per_frame: 400,
                iterations_per_frame: 50,
                colors: [
                    0xFF_000000, // 0 grains (Black)
                    0xFF_111166, // 1 grain (Dark Blue)
                    0xFF_2255AA, // 2 grains (Medium Blue)
                    0xFF_55AAAA, // 3 grains (Cyan)
                ],
            },
            frames: 0,
            presenter: None,
            fb: None,
        }
    }
}

impl WindowApp for SandpileDemo {
    type Error = SandpileError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Abelian Sandpile Filter".to_string(),
            width: 600,
            height: 600,
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(
            SoftwarePresenter::new(ctx.window.clone()).map_err(|e| SandpileError(e.to_string()))?,
        );
        let size = ctx.window.inner_size();
        self.fb = Some(Framebuffer::new(size.width, size.height).unwrap());
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.fb = Some(Framebuffer::new(width, height).unwrap());
        Ok(())
    }

    fn input(&mut self, ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput {
            event: kb_event, ..
        } = event
        {
            if kb_event.state.is_pressed() && kb_event.logical_key == Key::Named(NamedKey::Escape) {
                ctx.event_loop.exit();
            }
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if let Some(fb) = self.fb.as_mut() {
            // Set drop coordinate to the center
            self.config.drop_x = (fb.width() / 2) as usize;
            self.config.drop_y = (fb.height() / 2) as usize;

            // Apply sandpile effect directly onto the framebuffer
            apply_sandpile(fb, &self.config);

            if let Some(presenter) = self.presenter.as_mut() {
                presenter
                    .present(fb)
                    .map_err(|e| SandpileError(e.to_string()))?;
            }
        }

        self.frames += 1;
        Ok(())
    }
}

fn main() {
    let app = SandpileDemo::new();
    run_windowed(app);
}
