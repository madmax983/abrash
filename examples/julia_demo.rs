use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::julia::{JuliaConfig, render_julia};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Julia Set Demo";

struct JuliaDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    config: JuliaConfig,
    time: f64,
}

impl JuliaDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            config: JuliaConfig::default(),
            time: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for JuliaDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += f64::from(ctx.dt_seconds.max(0.0));

        // Slowly morph the c parameter to animate the Julia set
        self.config.c_re = -0.7 + 0.1 * (self.time * 0.5).sin();
        self.config.c_im = 0.27015 + 0.1 * (self.time * 0.3).cos();

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        render_julia(&mut self.framebuffer, &self.config);
        self.present()
    }
}

fn main() {
    run_windowed(JuliaDemo::new().unwrap());
}
