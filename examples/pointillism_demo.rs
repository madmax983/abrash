use abrash::experimental::pointillism::{PointillismConfig, apply_pointillism};
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "backend-winit")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::texture::Texture;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Pointillism Demo";

#[cfg(feature = "backend-winit")]
struct PointillismDemo {
    presenter: Option<SoftwarePresenter>,
    image: Framebuffer,
    config: PointillismConfig,
}

#[cfg(feature = "backend-winit")]
impl PointillismDemo {
    #[allow(clippy::unnecessary_wraps)]
    fn new() -> Result<Self, HostError> {
        let texture = Texture::checkered(256, 256, 0xFF_FF_00_00, 0xFF_00_FF_00).unwrap();
        let mut fb = Framebuffer::new(texture.width, texture.height).unwrap();
        fb.as_mut_slice().copy_from_slice(&texture.pixels);

        Ok(Self {
            presenter: None,
            image: fb,
            config: PointillismConfig {
                max_radius: 6.0,
                density: 4,
            },
        })
    }
}

#[cfg(feature = "backend-winit")]
impl WindowApp for PointillismDemo {
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

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        // Handle resizing the image
        let mut resized = Framebuffer::new(width, height).unwrap();
        for y in 0..height {
            for x in 0..width {
                let sx = (x * self.image.width() / width) as usize;
                let sy = (y * self.image.height() / height) as usize;
                let color = self.image.as_slice()[sy * self.image.width() as usize + sx];
                resized.as_mut_slice()[(y * width + x) as usize] = color;
            }
        }
        self.image = resized;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let result = apply_pointillism(&self.image, self.config);

        let presenter = self.presenter.as_mut().unwrap();
        presenter.present(&result)?;
        Ok(())
    }
}

#[cfg(feature = "backend-winit")]
fn main() {
    let app = PointillismDemo::new().unwrap();
    run_windowed(app);
}

#[cfg(not(feature = "backend-winit"))]
fn main() {
    println!("This example requires the 'backend-winit' feature.");
}
