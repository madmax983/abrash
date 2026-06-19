use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::process;

#[cfg(feature = "nova")]
use abrash::experimental::strange_attractor::{StrangeAttractor, StrangeAttractorConfig};

#[cfg(feature = "nova")]
struct StrangeAttractorDemo {
    framebuffer: Framebuffer,
    attractor: StrangeAttractor,
    config: StrangeAttractorConfig,
    presenter: Option<SoftwarePresenter>,

    // Animation and morphing state
    time: f32,
    base_a: f64,
    base_b: f64,
    base_c: f64,
    base_d: f64,
}

#[cfg(feature = "nova")]
impl StrangeAttractorDemo {
    fn new(width: u32, height: u32) -> Result<Self, String> {
        let framebuffer = Framebuffer::new(width, height)?;
        let attractor = StrangeAttractor::new(width as usize, height as usize);
        let config = StrangeAttractorConfig::default();

        Ok(Self {
            framebuffer,
            attractor,
            config,
            presenter: None,
            time: 0.0,
            base_a: 1.4,
            base_b: 1.56,
            base_c: 1.4,
            base_d: -0.56,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for StrangeAttractorDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            title: "Abrash - Strange Attractor Demo".to_string(),
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;

        // Slowly morph the parameters over time to animate the attractor
        self.config.a = self.base_a + f64::from((self.time * 0.1).sin()) * 0.2;
        self.config.b = self.base_b + f64::from((self.time * 0.15).cos()) * 0.2;
        self.config.c = self.base_c + f64::from((self.time * 0.07).sin()) * 0.15;
        self.config.d = self.base_d + f64::from((self.time * 0.12).cos()) * 0.15;

        // Change color based on time
        self.config.color = (
            (128.0 + (self.time * 0.5).sin() * 127.0) as u8,
            (128.0 + (self.time * 0.7).sin() * 127.0) as u8,
            (128.0 + (self.time * 0.3).sin() * 127.0) as u8,
        );

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_00_00_00);

        self.attractor.render(&mut self.framebuffer, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }

    fn resize(&mut self, _ctx: WindowContext<'_>, width: u32, height: u32) -> Result<(), Self::Error> {
        if width > 0 && height > 0 {
            self.framebuffer = Framebuffer::new(width, height).unwrap();
            self.attractor.resize(width as usize, height as usize);
        }
        Ok(())
    }
}

#[cfg(feature = "nova")]
fn main() {
    match StrangeAttractorDemo::new(800, 600) {
        Ok(app) => run_windowed(app),
        Err(e) => {
            eprintln!("Failed to initialize app: {e}");
            process::exit(1);
        }
    }
}

#[cfg(not(feature = "nova"))]
fn main() {
    println!("This example requires the 'nova' feature. Run with --features nova");
}
