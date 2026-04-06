use abrash::experimental::frosted_glass::apply_frosted_glass;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::texture::Texture;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

struct FrostedGlassDemo {
    fb: Framebuffer,
    image: Texture,
    radius: u32,
    presenter: Option<SoftwarePresenter>,
}

impl FrostedGlassDemo {
    fn new(width: u32, height: u32) -> Self {
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF222222);

        // Create a simple procedural image to look through
        let mut image = Texture::new(width, height).unwrap();
        let w = width as usize;
        let h = height as usize;
        let buffer = image.pixels_mut();

        for y in 0..h {
            for x in 0..w {
                let r = ((x * 255) / w) as u32;
                let g = ((y * 255) / h) as u32;
                let b = (((x ^ y) * 255) / w.max(h)) as u32;
                buffer[y * w + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }

        // Draw some sharp rectangles over the gradient to show the distortion better
        for y in (h / 4)..(h * 3 / 4) {
            for x in (w / 4)..(w * 3 / 4) {
                if (x / 20) % 2 == (y / 20) % 2 {
                    buffer[y * w + x] = 0xFFFFFFFF;
                }
            }
        }

        Self {
            fb,
            image,
            radius: 10,
            presenter: None,
        }
    }
}

impl WindowApp for FrostedGlassDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Frosted Glass Filter Demo (Up/Down to change radius)".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy original image
        let dst = self.fb.as_mut_slice();
        let src = self.image.pixels();
        dst.copy_from_slice(src);

        // Apply frosted glass effect
        apply_frosted_glass(&mut self.fb, self.radius);

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state.is_pressed() {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::ArrowUp => self.radius = self.radius.saturating_add(1),
                        KeyCode::ArrowDown => self.radius = self.radius.saturating_sub(1),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() {
    let app = FrostedGlassDemo::new(WIDTH, HEIGHT);
    run_windowed(app);
}
