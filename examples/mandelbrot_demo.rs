#[cfg(feature = "nova")]
pub mod app {
    use super::*;

    use abrash::framebuffer::Framebuffer;
    use abrash::platform::{
        HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
    };

    use abrash::experimental::mandelbrot::{MandelbrotConfig, render_mandelbrot};

    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    struct MandelbrotDemo {
        presenter: Option<SoftwarePresenter>,
        framebuffer: Framebuffer,
        #[cfg(feature = "nova")]
        config: MandelbrotConfig,
    }

    impl MandelbrotDemo {
        fn new() -> Result<Self, HostError> {
            Ok(Self {
                presenter: None,
                framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                    .map_err(|error| HostError::App(error.to_string()))?,
                #[cfg(feature = "nova")]
                config: MandelbrotConfig::default(),
            })
        }
    }

    impl WindowApp for MandelbrotDemo {
        type Error = HostError;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: "🌟 Nova: Mandelbrot Demo".to_string(),
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
            self.framebuffer = Framebuffer::new(width, height)
                .map_err(|error| HostError::App(error.to_string()))?;
            Ok(())
        }

        fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            // Here we would handle input for zooming/panning
            // To keep it simple for now, we'll auto-zoom
            #[cfg(feature = "nova")]
            {
                self.config.zoom *= 1.01;
                // self.config.center_x += 0.001 / self.config.zoom;
            }
            Ok(())
        }

        fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            #[cfg(feature = "nova")]
            render_mandelbrot(&mut self.framebuffer, &self.config);

            let framebuffer = &self.framebuffer;
            let presenter = self.presenter.as_mut().ok_or_else(|| {
                HostError::Present("software presenter not initialized".to_string())
            })?;
            presenter.present(framebuffer)?;
            Ok(())
        }
    }

    // Fallback for when "nova" feature is not enabled

    pub fn main() {
        println!("🌟 Nova: Mandelbrot Demo");
        run_windowed(MandelbrotDemo::new().unwrap());
    }
}

#[cfg(feature = "nova")]
fn main() {
    app::main();
}

#[cfg(not(feature = "nova"))]
fn main() {
    let mut error_table = comfy_table::Table::new();
    error_table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            comfy_table::Cell::new("⚠️  Missing Feature: Nova")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Red),
        ])
        .add_row(vec![
            comfy_table::Cell::new("This example requires the 'nova' feature to run.")
                .fg(comfy_table::Color::White),
        ])
        .add_row(vec![
            comfy_table::Cell::new(
                "Try running with:\ncargo run --example mandelbrot_demo --features nova",
            )
            .fg(comfy_table::Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}
