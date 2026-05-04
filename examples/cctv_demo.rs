use abrash::experimental::cctv::{CctvConfig, apply_cctv};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
#[cfg(feature = "backend-tui")]
use abrash::platform::tui::VirtualKeyCode;
use abrash::platform::{DemoExt, EngineAdapter, demo_main};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
#[cfg(feature = "backend-winit")]
use winit::event::VirtualKeyCode;

struct CctvDemo {
    fb: Framebuffer,
    zb: ZBuffer,
    angle: f32,
    cctv_config: CctvConfig,
}

impl CctvDemo {
    fn new(width: u32, height: u32) -> Self {
        Self {
            fb: Framebuffer::new(width, height).unwrap(),
            zb: ZBuffer::new(width, height).unwrap(),
            angle: 0.0,
            cctv_config: CctvConfig::default(),
        }
    }
}

impl EngineAdapter for CctvDemo {
    fn update(&mut self, dt: f32) {
        self.angle += dt * 0.5;
        self.cctv_config.time += dt;

        self.fb.clear(0xFF_444444);
        self.zb.clear();

        let eye = Vec3::new(0.0, 2.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(
            1.0,
            self.fb.width() as f32 / self.fb.height() as f32,
            0.1,
            100.0,
        );

        let rot_y = Mat4::rotation_y(self.angle);
        let rot_x = Mat4::rotation_x(self.angle * 0.5);
        let world = rot_y * rot_x;

        let wvp = world * view * proj;

        // Draw a simple colorful triangle
        let v0 = Vec3::new(0.0, 1.0, 0.0);
        let v1 = Vec3::new(-1.0, -1.0, 0.0);
        let v2 = Vec3::new(1.0, -1.0, 0.0);

        let v0_clip = wvp.transform_point(v0);
        let v1_clip = wvp.transform_point(v1);
        let v2_clip = wvp.transform_point(v2);

        fill_triangle_3d(
            &mut self.fb,
            &mut self.zb,
            v0_clip,
            v1_clip,
            v2_clip,
            0xFFFF0000,
        );

        // Apply CCTV filter
        apply_cctv(&mut self.fb, &self.cctv_config);
    }

    fn draw(&self, dest: &mut [u32], pitch: usize) {
        let src = self.fb.as_slice();
        let w = self.fb.width() as usize;
        let h = self.fb.height() as usize;
        for y in 0..h {
            dest[y * pitch..y * pitch + w].copy_from_slice(&src[y * w..y * w + w]);
        }
    }
}

impl DemoExt for CctvDemo {
    fn name(&self) -> &'static str {
        "CCTV Camera Filter Demo"
    }

    fn description(&self) -> &'static str {
        "A retro post-processing effect simulating a cheap security camera feed."
    }

    #[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
    fn handle_key(&mut self, keycode: VirtualKeyCode, _state: abrash::platform::ElementState) {
        match keycode {
            VirtualKeyCode::Up => {
                self.cctv_config.noise_intensity =
                    (self.cctv_config.noise_intensity + 0.05).min(1.0)
            }
            VirtualKeyCode::Down => {
                self.cctv_config.noise_intensity =
                    (self.cctv_config.noise_intensity - 0.05).max(0.0)
            }
            VirtualKeyCode::Right => {
                self.cctv_config.desaturation = (self.cctv_config.desaturation + 0.05).min(1.0)
            }
            VirtualKeyCode::Left => {
                self.cctv_config.desaturation = (self.cctv_config.desaturation - 0.05).max(0.0)
            }
            _ => {}
        }
    }
}

fn main() {
    let demo = CctvDemo::new(800, 600);
    demo_main(demo);
}
