use abrash::experimental::neon_outline::{NeonOutlineConfig, apply_neon_outline};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Config, WindowApp, run_windowed};
use abrash::rasterizer::RasterizerConfig;
use abrash::scene::Scene;
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

struct NeonOutlineDemoApp {
    fb: Framebuffer,
    zb: ZBuffer,
    scene: Scene,
    mesh: Arc<Mesh>,
    camera_pos: Vec3,
    yaw: f32,
    pitch: f32,
    light_dir: Vec3,
    config: NeonOutlineConfig,
    show_effect: bool,
    time: f32,
}

impl NeonOutlineDemoApp {
    pub fn new(width: usize, height: usize) -> Self {
        let mut scene = Scene::new(width, height);

        let mesh = Arc::new(Mesh::cube());

        Self {
            fb: Framebuffer::new(width, height).unwrap(),
            zb: ZBuffer::new(width, height).unwrap(),
            scene,
            mesh,
            camera_pos: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            light_dir: Vec3::new(0.0, 0.0, -1.0).normalize(),
            config: NeonOutlineConfig::default(),
            show_effect: true,
            time: 0.0,
        }
    }
}

impl WindowApp for NeonOutlineDemoApp {
    fn config() -> Config {
        Config {
            width: 800,
            height: 600,
            title: "Abrash - Neon Outline Filter Demo".to_string(),
            ..Default::default()
        }
    }

    fn init(&mut self, _presenter: &mut abrash::platform::SoftwarePresenter) {}

    fn update(&mut self, dt: f32) {
        self.time += dt;

        self.scene.clear();

        // Rotate the mesh
        let model_matrix = Mat4::rotation_x(self.time * 0.5)
            * Mat4::rotation_y(self.time * 0.7)
            * Mat4::rotation_z(self.time * 0.3)
            * Mat4::scale(Vec3::new(1.5, 1.5, 1.5));

        self.scene.add_mesh(self.mesh.clone(), model_matrix);

        // Update Camera
        let view_matrix = Mat4::look_at(self.camera_pos, Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
        let proj_matrix = Mat4::perspective(
            1.047, // 60 degrees FoV
            self.fb.width() as f32 / self.fb.height() as f32,
            0.1,
            100.0,
        );

        self.scene
            .set_camera(view_matrix, proj_matrix, self.camera_pos);
        self.scene.set_directional_light(self.light_dir);
    }

    fn render(&mut self, _dt: f32) {
        self.fb.clear(0xFF_111111);
        self.zb.clear();

        let mut config = RasterizerConfig::default();
        config.flat_shading = true; // Use flat shading to create strong edges

        self.scene.render(&mut self.fb, &mut self.zb, &config, 0.0);

        if self.show_effect {
            // Pulse the threshold and colors slightly for effect
            let mut fx_config = self.config;
            fx_config.threshold = (40.0 + (self.time * 2.0).sin() * 10.0) as u32;

            // Cycle the horizontal color
            let r = ((self.time).sin() * 127.0 + 128.0) as u32;
            let g = ((self.time + 2.0).sin() * 127.0 + 128.0) as u32;
            let b = ((self.time + 4.0).sin() * 127.0 + 128.0) as u32;
            fx_config.color_horizontal = 0xFF_000000 | (r << 16) | (g << 8) | b;

            apply_neon_outline(&mut self.fb, &fx_config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state.is_pressed() {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Space) => {
                        self.show_effect = !self.show_effect;
                        println!("Neon Outline Effect: {}", self.show_effect);
                    }
                    _ => {}
                }
            }
        }
        false
    }
}

fn main() {
    let app = NeonOutlineDemoApp::new(800, 600);
    run_windowed(app);
}
