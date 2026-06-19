//! Cloth Simulation Demo
//!
//! Visualizes a mass-spring cloth simulation.
//! Requires the `nova` feature.

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::error::Error;

#[cfg(all(feature = "nova", not(feature = "backend-winit")))]
mod demo {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::platform::Window;
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::zbuffer::ZBuffer;
    use abrash_render::experimental::cloth::Cloth;
    use std::f32::consts::PI;
    use std::time::Instant;

    const WIDTH: usize = 800;
    const HEIGHT: usize = 600;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut window = Window::new("Abrash - Cloth Simulation", WIDTH as u32, HEIGHT as u32)?;
        let mut fb = Framebuffer::new(WIDTH as u32, HEIGHT as u32)?;
        let mut zb = ZBuffer::new(WIDTH as u32, HEIGHT as u32)?;

        // Initialize Cloth
        // 20x20 grid, spacing 0.2 -> 4.0x4.0 size
        let mut cloth = Cloth::new(20, 20, 0.2);

        // Pin the top corners
        cloth.pin(0, 0); // Top-Left (actually y=0 is bottom in my coord sys? Let's check)
        // In Cloth::new:
        // y * spacing - (height * spacing) / 2.0
        // If height=20, spacing=0.2. Total height 4.0. Offset -2.0.
        // y=0 -> -2.0 (Bottom). y=19 -> 1.8 (Top).
        // So y=height-1 is top.
        cloth.pin(0, cloth.height - 1);
        cloth.pin(cloth.width - 1, cloth.height - 1);
        // Pin middle too for a "curtain" look
        cloth.pin(cloth.width / 2, cloth.height - 1);

        let gravity = Vec3::new(0.0, -9.8, 0.0);
        let mut wind = Vec3::new(0.0, 0.0, 2.0); // Blowing towards Z

        let mut last_frame = Instant::now();
        let mut time = 0.0;

        // Camera
        let proj = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 6.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let view_proj = view * proj; // Row-major: v * View * Proj

        while window.is_open() {
            window.poll_events();

            let now = Instant::now();
            let dt = (now - last_frame).as_secs_f32();
            last_frame = now;
            time += dt;

            // Vary wind
            wind.x = (time * 2.0).sin() * 2.0;
            wind.z = 2.0 + (time * 1.5).cos() * 1.0;

            // Update Physics
            // Sub-step for stability
            let sub_steps = 5;
            let sub_dt = dt.min(0.032) / sub_steps as f32; // Cap dt to avoid explosion
            for _ in 0..sub_steps {
                cloth.update(sub_dt, gravity, wind);
            }

            // Render
            fb.clear(0xFF101010); // Dark Gray
            zb.clear();

            let mesh = cloth.to_mesh();

            // Light direction (from camera roughly)
            let light_dir = Vec3::new(0.5, 1.0, 1.0).normalize();

            for tri in &mesh.indices {
                let i0 = tri[0];
                let i1 = tri[1];
                let i2 = tri[2];

                let v0 = mesh.vertices[i0];
                let v1 = mesh.vertices[i1];
                let v2 = mesh.vertices[i2];

                // Compute face normal for flat shading
                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                let normal = edge1.cross(edge2).normalize();

                // Simple Lambertian
                let ndotl = normal.dot(light_dir).max(0.1);

                // Color: Red Cloth (0xFFAA0000)
                let r = (170.0 * ndotl) as u32;
                let g = (20.0 * ndotl) as u32;
                let b = (20.0 * ndotl) as u32;
                let color = 0xFF000000 | (r << 16) | (g << 8) | b;

                let (c0, w0) = view_proj.transform_point(v0);
                let (c1, w1) = view_proj.transform_point(v1);
                let (c2, w2) = view_proj.transform_point(v2);

                // Simple clipping check
                if w0 < 0.1 || w1 < 0.1 || w2 < 0.1 {
                    continue;
                }

                fill_triangle_3d(&mut fb, &mut zb, (c0, w0), (c1, w1), (c2, w2), color);
            }

            window.blit_framebuffer(&fb);
        }

        Ok(())
    }
}

#[cfg(all(feature = "nova", feature = "backend-winit"))]
mod winit_demo {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::platform::{
        SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
    };
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::zbuffer::ZBuffer;
    use abrash_render::experimental::cloth::Cloth;
    use std::f32::consts::PI;
    use std::io;

    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        match ClothApp::new() {
            Ok(app) => run_windowed(app),
            Err(e) => {
                let mut error_table = comfy_table::Table::new();
                error_table
                    .load_preset(comfy_table::presets::UTF8_FULL)
                    .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        comfy_table::Cell::new("❌ Initialization Error")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(comfy_table::Color::Red),
                    ])
                    .add_row(vec![
                        comfy_table::Cell::new(format!("{e}")).fg(comfy_table::Color::Yellow),
                    ]);
                eprintln!("\n{error_table}");
                std::process::exit(1);
            }
        }
        Ok(())
    }

    struct ClothApp {
        width: u32,
        height: u32,
        framebuffer: Framebuffer,
        zbuffer: ZBuffer,
        presenter: Option<SoftwarePresenter>,
        cloth: Cloth,
        gravity: Vec3,
        wind: Vec3,
        time_seconds: f32,
    }

    impl ClothApp {
        fn new() -> Result<Self, io::Error> {
            let mut cloth = Cloth::new(20, 20, 0.2);
            cloth.pin(0, cloth.height - 1);
            cloth.pin(cloth.width - 1, cloth.height - 1);
            cloth.pin(cloth.width / 2, cloth.height - 1);

            let width = WIDTH;
            let height = HEIGHT;

            let framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            let zbuffer = ZBuffer::new(width, height).map_err(io::Error::other)?;

            Ok(Self {
                width,
                height,
                framebuffer,
                zbuffer,
                presenter: None,
                cloth,
                gravity: Vec3::new(0.0, -9.8, 0.0),
                wind: Vec3::new(0.0, 0.0, 2.0),
                time_seconds: 0.0,
            })
        }

        fn rebuild_buffers(&mut self, width: u32, height: u32) -> Result<(), io::Error> {
            self.width = width;
            self.height = height;
            self.framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            self.zbuffer = ZBuffer::new(width, height).map_err(io::Error::other)?;
            Ok(())
        }

        fn render_cloth(&mut self) {
            self.framebuffer.clear(0xFF10_1010);
            self.zbuffer.clear();

            let projection =
                Mat4::perspective(PI / 3.0, self.width as f32 / self.height as f32, 0.1, 100.0);
            let view = Mat4::look_at(
                Vec3::new(0.0, 2.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            let view_proj = view * projection;
            let light_dir = Vec3::new(0.5, 1.0, 1.0).normalize();
            let mesh = self.cloth.to_mesh();

            for tri in &mesh.indices {
                let v0 = mesh.vertices[tri[0]];
                let v1 = mesh.vertices[tri[1]];
                let v2 = mesh.vertices[tri[2]];

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                let normal = edge1.cross(edge2).normalize();
                let diffuse = normal.dot(light_dir).max(0.1);

                let r = (170.0 * diffuse) as u32;
                let g = (20.0 * diffuse) as u32;
                let b = (20.0 * diffuse) as u32;
                let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                let (c0, w0) = view_proj.transform_point(v0);
                let (c1, w1) = view_proj.transform_point(v1);
                let (c2, w2) = view_proj.transform_point(v2);

                if w0 < 0.1 || w1 < 0.1 || w2 < 0.1 {
                    continue;
                }

                fill_triangle_3d(
                    &mut self.framebuffer,
                    &mut self.zbuffer,
                    (c0, w0),
                    (c1, w1),
                    (c2, w2),
                    color,
                );
            }
        }
    }

    impl WindowApp for ClothApp {
        type Error = io::Error;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: "Abrash - Cloth Simulation".to_string(),
                width: self.width,
                height: self.height,
                vsync: true,
            }
        }

        fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            self.presenter = Some(SoftwarePresenter::new(ctx.window).map_err(io::Error::other)?);
            Ok(())
        }

        fn resize(
            &mut self,
            _ctx: WindowContext<'_>,
            width: u32,
            height: u32,
        ) -> Result<(), Self::Error> {
            self.rebuild_buffers(width, height)
        }

        fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            let dt = ctx.dt_seconds.min(0.032);
            self.time_seconds += dt;
            self.wind.x = (self.time_seconds * 2.0).sin() * 2.0;
            self.wind.z = 2.0 + (self.time_seconds * 1.5).cos();

            let sub_steps = 5;
            let sub_dt = dt / sub_steps as f32;
            for _ in 0..sub_steps {
                self.cloth.update(sub_dt, self.gravity, self.wind);
            }

            Ok(())
        }

        fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            self.render_cloth();
            let presenter = self
                .presenter
                .as_mut()
                .ok_or_else(|| io::Error::other("presenter not initialized"))?;
            presenter
                .present(&self.framebuffer)
                .map_err(io::Error::other)
        }
    }
}

fn print_banner() {
    println!("\n{}", "👗 Cloth Simulation Demo".bold().magenta());
    println!("{}", "========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Feature").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Soft Body"),
            Cell::new("Mass-Spring System").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Physics"),
            Cell::new("Wind Simulation").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Rendering"),
            Cell::new("Software Rasterizer + Flat Shading").fg(Color::Blue),
        ]);

    println!("\n{}", "⚙️  System Info".bold());
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
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("None (Passive Simulation)"),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Q / Esc to Quit")]);
    println!("{controls}\n");
}

#[cfg(feature = "backend-winit")]
fn main() -> Result<(), Box<dyn Error>> {
    print_banner();

    #[cfg(feature = "nova")]
    {
        winit_demo::run()
    }

    #[cfg(not(feature = "nova"))]
    {
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("⚠️  Missing Feature: Nova")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
            ])
            .add_row(vec![
                Cell::new("This demo requires the 'nova' feature to run.").fg(Color::White),
            ])
            .add_row(vec![
                Cell::new("Try running with:\ncargo run --example cloth_demo --features nova")
                    .fg(Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "backend-winit"))]
fn main() -> Result<(), Box<dyn Error>> {
    print_banner();

    #[cfg(feature = "nova")]
    {
        demo::run()
    }
    #[cfg(not(feature = "nova"))]
    {
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("⚠️  Missing Feature: Nova")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
            ])
            .add_row(vec![
                Cell::new("This demo requires the 'nova' feature to run.").fg(Color::White),
            ])
            .add_row(vec![
                Cell::new("Try running with:\ncargo run --example cloth_demo --features nova")
                    .fg(Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
