use abrash::experimental::pop_art::{PopArtConfig, apply_pop_art};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{WindowApp, WindowContext, run_windowed};
use abrash::rasterizer::SoftwarePresenter;
use abrash::zbuffer::ZBuffer;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

struct PopArtApp {
    presenter: SoftwarePresenter,
    fb: Framebuffer,
    zb: ZBuffer,
    angle: f32,
    config: PopArtConfig,
}

impl PopArtApp {
    fn new(width: u32, height: u32) -> Self {
        Self {
            presenter: SoftwarePresenter::new(width, height),
            fb: Framebuffer::new(width, height).unwrap(),
            zb: ZBuffer::new(width, height).unwrap(),
            angle: 0.0,
            config: PopArtConfig::default(),
        }
    }
}

impl WindowApp for PopArtApp {
    type Error = String;

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.angle += ctx.dt() * 1.5;

        let width = self.fb.width() as f32;
        let height = self.fb.height() as f32;

        self.fb.clear(0xFF222222);
        self.zb.clear();

        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_3, width / height, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let model1 = Mat4::rotate_y(self.angle) * Mat4::translate(Vec3::new(-1.0, 0.0, 0.0));
        let model2 = Mat4::rotate_x(self.angle * 0.7) * Mat4::translate(Vec3::new(1.0, 0.0, 0.0));

        let mvp1 = model1 * view * proj;
        let mvp2 = model2 * view * proj;

        // Draw a simple triangle for model 1
        let v0_1 = mvp1.transform_point(Vec3::new(0.0, 1.0, 0.0));
        let v1_1 = mvp1.transform_point(Vec3::new(-1.0, -1.0, 0.0));
        let v2_1 = mvp1.transform_point(Vec3::new(1.0, -1.0, 0.0));
        abrash::rasterizer::fill_triangle_3d(
            &mut self.fb,
            &mut self.zb,
            v0_1,
            v1_1,
            v2_1,
            0xFFFFFFFF,
        );

        // Draw a simple triangle for model 2
        let v0_2 = mvp2.transform_point(Vec3::new(0.0, -1.0, 0.0));
        let v1_2 = mvp2.transform_point(Vec3::new(1.0, 1.0, 0.0));
        let v2_2 = mvp2.transform_point(Vec3::new(-1.0, 1.0, 0.0));
        abrash::rasterizer::fill_triangle_3d(
            &mut self.fb,
            &mut self.zb,
            v0_2,
            v1_2,
            v2_2,
            0xFFBBBBBB,
        );

        // Apply Pop Art filter
        apply_pop_art(&mut self.fb, &self.config);

        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter.present(ctx.window, &self.fb);
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput { event: kb, .. } = event {
            if kb.state.is_pressed() {
                if let PhysicalKey::Code(KeyCode::Space) = kb.physical_key {
                    // Randomize colors on space
                    self.config.color_top_left = 0xFF000000 | rand::random::<u32>();
                    self.config.color_top_right = 0xFF000000 | rand::random::<u32>();
                    self.config.color_bottom_left = 0xFF000000 | rand::random::<u32>();
                    self.config.color_bottom_right = 0xFF000000 | rand::random::<u32>();
                }
            }
        }
        Ok(())
    }
}

fn main() {
    let app = PopArtApp::new(800, 600);
    run_windowed(app, "Pop Art Demo - Space to randomize colors").unwrap();
}
