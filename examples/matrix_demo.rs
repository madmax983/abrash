//! Matrix Digital Rain Demo
//!
//! A retro post-processing effect simulating the classic "Digital Rain"
//! overlay.

#[cfg(feature = "nova")]
use abrash::experimental::matrix_rain::{MatrixRainConfig, apply_matrix_rain};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::time::Instant;

struct MatrixDemo {
    framebuffer: Framebuffer,
    start_time: Instant,
    presenter: Option<SoftwarePresenter>,
    #[cfg(feature = "nova")]
    config: MatrixRainConfig,
}

impl MatrixDemo {
    fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: Framebuffer::new(width, height).unwrap(),
            start_time: Instant::now(),
            presenter: None,
            #[cfg(feature = "nova")]
            config: MatrixRainConfig::default(),
        }
    }
}

impl WindowApp for MatrixDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Matrix Digital Rain".to_string(),
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
        if width > 0
            && height > 0
            && (width != self.framebuffer.width() || height != self.framebuffer.height())
        {
            self.framebuffer = Framebuffer::new(width, height).unwrap();
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        #[cfg(feature = "nova")]
        {
            self.config.time = self.start_time.elapsed().as_secs_f32();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        #[cfg(feature = "nova")]
        {
            // The matrix effect handles its own clearing/fading internally
            apply_matrix_rain(&mut self.framebuffer, &self.config);
        }

        #[cfg(not(feature = "nova"))]
        {
            self.framebuffer.clear(0xFF_000000); // Just clear to black if nova isn't enabled
        }

        if let Some(p) = &mut self.presenter {
            p.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    println!("🌟 Starting Nova: Matrix Digital Rain Demo 🌟");

    let width = 800;
    let height = 600;

    let app = MatrixDemo::new(width, height);
    run_windowed(app)
}
