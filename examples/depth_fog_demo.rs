#[cfg(feature = "nova")]
mod app {
    use abrash::anim::Timeline;
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::mesh::Mesh;
    use abrash::platform::{
        HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
    };
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::time::FixedTimestep;
    use abrash::zbuffer::ZBuffer;
    use abrash_render::experimental::depth_fog::{DepthFogConfig, apply_depth_fog};
    use std::f32::consts::PI;
    use std::time::Duration;

    use comfy_table::{Cell, Color, Table, presets};
    use crossterm::style::Stylize;

    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;
    const BACKGROUND: u32 = 0xFF_11_11_22;
    const TITLE: &str = "Abrash - Depth Fog Filter";

    // Face colors for the cube
    const COLORS: [u32; 6] = [
        0xFFFF_0000, // Red
        0xFF00_FF00, // Green
        0xFF00_00FF, // Blue
        0xFFFF_FF00, // Yellow
        0xFFFF_00FF, // Magenta
        0xFF00_FFFF, // Cyan
    ];

    fn print_banner() {
        println!("\n{}", "🌫️ Depth Fog Filter Demo".bold().cyan());
        println!("{}", "==========================".dark_grey());

        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Property").fg(Color::Cyan),
                Cell::new("Value").fg(Color::Cyan),
            ])
            .add_row(vec![
                Cell::new("Description"),
                Cell::new("Distance-based atmospheric fog based on Z-buffer.").fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new("Renderer"),
                Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
            ]);

        println!("\n{}", "⚙️  Info".bold());
        println!("{table}");

        println!("\n{}", "🎮 Controls".bold());
        let mut controls = Table::new();
        controls
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Input").fg(Color::Cyan),
                Cell::new("Action").fg(Color::Cyan),
            ])
            .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
            .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);
        println!("{controls}\n");
    }

    struct DepthFogApp {
        presenter: Option<SoftwarePresenter>,
        framebuffer: Framebuffer,
        zbuffer: ZBuffer,
        timestep: FixedTimestep,
        cube: Mesh,
        rotation_y: Timeline<f32>,
        rotation_x: Timeline<f32>,
    }

    impl DepthFogApp {
        fn new() -> Result<Self, HostError> {
            Ok(Self {
                presenter: None,
                framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                    .map_err(|error| HostError::App(error.to_string()))?,
                zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                    .map_err(|error| HostError::App(error.to_string()))?,
                timestep: FixedTimestep::new(60),
                cube: Mesh::cube(1.0),
                rotation_y: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(6))
                    .loop_forever(),
                rotation_x: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(12))
                    .loop_forever(),
            })
        }
    }

    impl WindowApp for DepthFogApp {
        type Error = HostError;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: TITLE.to_string(),
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
            if width > 0 && height > 0 {
                self.framebuffer = Framebuffer::new(width, height)
                    .map_err(|error| HostError::App(error.to_string()))?;
                self.zbuffer = ZBuffer::new(width, height)
                    .map_err(|error| HostError::App(error.to_string()))?;
            }
            Ok(())
        }

        fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            let steps = self.timestep.update();
            for _ in 0..steps {
                self.rotation_y.tick(self.timestep.dt());
                self.rotation_x.tick(self.timestep.dt());
            }
            Ok(())
        }

        fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            let projection = Mat4::perspective(
                PI / 3.0,
                self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
                0.1,
                100.0,
            );

            // Look slightly down at the cubes
            let view = Mat4::look_at(
                Vec3::new(0.0, 3.0, 10.0),
                Vec3::new(0.0, 0.0, -10.0),
                Vec3::new(0.0, 1.0, 0.0),
            );

            self.framebuffer.clear(BACKGROUND);
            self.zbuffer.clear();

            let ry = self.rotation_y.current_value();
            let rx = self.rotation_x.current_value();

            // Draw multiple cubes at different depth to demonstrate the fog
            for i in 0..6 {
                let z_offset = -(i as f32) * 5.0; // Place cubes increasingly far away
                let model = Mat4::translation((i as f32 - 2.5) * 2.0, 0.0, z_offset)
                    * Mat4::rotation_y(ry + i as f32 * 0.5)
                    * Mat4::rotation_x(rx + i as f32 * 0.3);

                let mvp = projection * (view * model);

                for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
                    let v0 = self.cube.vertices[tri_indices[0]];
                    let v1 = self.cube.vertices[tri_indices[1]];
                    let v2 = self.cube.vertices[tri_indices[2]];

                    let (clip0, w0) = mvp.transform_point(v0);
                    let (clip1, w1) = mvp.transform_point(v1);
                    let (clip2, w2) = mvp.transform_point(v2);

                    if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                        continue;
                    }

                    fill_triangle_3d(
                        &mut self.framebuffer,
                        &mut self.zbuffer,
                        (clip0, w0),
                        (clip1, w1),
                        (clip2, w2),
                        COLORS[face_idx / 2],
                    );
                }
            }

            // Apply Depth Fog post-process
            // Make the fog end around distance 30 where the furthest cube is
            let fog_config = DepthFogConfig {
                fog_color: BACKGROUND, // Blend into background color
                fog_start: 5.0,
                fog_end: 35.0,
            };

            apply_depth_fog(&mut self.framebuffer, &self.zbuffer, &fog_config);

            if let Some(presenter) = &mut self.presenter {
                presenter.present(&self.framebuffer)?;
            }

            Ok(())
        }
    }

    pub fn run() {
        print_banner();
        let app = DepthFogApp::new().expect("Failed to initialize app");
        let () = run_windowed(app);
    }
}

fn main() {
    #[cfg(feature = "nova")]
    app::run();

    #[cfg(not(feature = "nova"))]
    {
        use comfy_table::{Cell, Color, Table, presets};
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("⚠️  Missing Feature: Nova")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
                Cell::new("Required Flag").fg(Color::Cyan),
            ])
            .add_row(vec![
                Cell::new("Depth Fog Filter Demo requires the `nova` feature."),
                Cell::new("--features nova"),
            ]);
        eprintln!("\n{table}");
    }
}
