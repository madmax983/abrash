#![cfg(feature = "nova")]

use abrash::color::Color;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::lens_distortion::{apply_lens_distortion, LensDistortionConfig};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

struct LensDistortionApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    config: LensDistortionConfig,
    up_pressed: bool,
    down_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    shift_pressed: bool,
}

impl LensDistortionApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e:?}")))?;

        let mut original_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create original framebuffer: {e:?}")))?;

        // Draw a checkboard pattern with lines to clearly show distortion
        original_fb.clear(Color::WHITE.to_argb_u32());

        let spacing = 40;
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if x % spacing == 0 || y % spacing == 0 {
                    original_fb.set_pixel(x, y, Color::BLACK.to_argb_u32());
                } else if (x / spacing) % 2 == (y / spacing) % 2 {
                    original_fb.set_pixel(x, y, 0xFF_DD_DD_DD); // Light gray squares
                }
            }
        }

        // Draw center dot
        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        for y in cy - 5..=cy + 5 {
            for x in cx - 5..=cx + 5 {
                original_fb.set_pixel(x, y, Color::RED.to_argb_u32());
            }
        }

        Ok(Self {
            framebuffer,
            original_fb,
            presenter: None,
            config: LensDistortionConfig {
                distortion: 0.5, // Start with pincushion
                scale: 0.8,
            },
            up_pressed: false,
            down_pressed: false,
            left_pressed: false,
            right_pressed: false,
            shift_pressed: false,
        })
    }
}

impl WindowApp for LensDistortionApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Lens Distortion Demo (Up/Down: Distortion, Left/Right: Scale)".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key_code),
                    state,
                    ..
                },
            ..
        } = event
        {
            let is_pressed = *state == ElementState::Pressed;
            match key_code {
                KeyCode::ArrowUp => self.up_pressed = is_pressed,
                KeyCode::ArrowDown => self.down_pressed = is_pressed,
                KeyCode::ArrowLeft => self.left_pressed = is_pressed,
                KeyCode::ArrowRight => self.right_pressed = is_pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.shift_pressed = is_pressed,
                _ => {}
            }
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let mut speed = 0.01;

        if self.shift_pressed {
            speed *= 5.0;
        }

        // Adjust distortion (k)
        if self.up_pressed {
            self.config.distortion += speed;
        }
        if self.down_pressed {
            self.config.distortion -= speed;
        }

        // Adjust scale
        if self.right_pressed {
            self.config.scale += speed;
        }
        if self.left_pressed {
            self.config.scale -= speed;
            if self.config.scale < 0.1 {
                self.config.scale = 0.1;
            }
        }

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Reset framebuffer manually using slice copy
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        // Apply distortion
        apply_lens_distortion(&mut self.framebuffer, self.config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() {
    run_windowed(LensDistortionApp::new().unwrap())
}
