use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::error::Error;

#[cfg(all(feature = "nova", not(feature = "backend-winit")))]
mod demo {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::mesh::Mesh;
    use abrash::platform::Window;
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::zbuffer::ZBuffer;
    use abrash_render::experimental::jelly::SoftBody;
    use abrash_render::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene, render_sdf};
    use std::time::Instant;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let width = 800;
        let height = 600;

        let mut window = Window::new("Jelly Physics Demo", width, height)?;
        let mut fb = Framebuffer::new(width, height)?;
        let mut zb = ZBuffer::new(width, height)?;

        // Scene Setup
        let mut sdf_scene = SdfScene::new();
        // Floor
        sdf_scene.add(SdfObject {
            primitive: SdfPrimitive::Plane {
                normal: Vec3::new(0.0, 1.0, 0.0),
                distance: 2.0, // y = -2.0
            },
            color: 0xFF555555,
        });
        // Sphere
        sdf_scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.5,
                center: Vec3::new(0.0, -2.0, 0.0),
            },
            color: 0xFF0000FF,
        });

        // Jelly Setup
        let mut mesh = Mesh::cube(2.0);
        for v in &mut mesh.vertices {
            v.y += 5.0; // Start high
        }

        let mut jelly = SoftBody::new(mesh, 1.0, 150.0, 2.0).expect("Failed to create jelly");

        // Add internal cross-bracing springs for stability
        let diag_len = (jelly.mesh.vertices[0] - jelly.mesh.vertices[6]).length();
        jelly.add_spring(0, 6, diag_len).unwrap();
        jelly.add_spring(1, 7, diag_len).unwrap();
        jelly.add_spring(2, 4, diag_len).unwrap();
        jelly.add_spring(3, 5, diag_len).unwrap();

        // Camera
        let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 12.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let mvp = proj * view; // Standard transformation order

        let mut last_time = Instant::now();

        while window.is_open() {
            window.poll_events();

            let now = Instant::now();
            let dt = (now - last_time).as_secs_f32().min(0.05);
            last_time = now;

            // Physics Update
            for _ in 0..4 {
                jelly.update(dt / 4.0);
                jelly.collide_sdf(&sdf_scene, 0.7);
            }

            fb.clear(0xFF101010);
            zb.clear();

            // 1. Render SDF obstacles
            render_sdf(
                &mut fb,
                &mut zb,
                &sdf_scene,
                &view,
                &proj,
                Vec3::new(0.0, 2.0, 12.0),
            );

            // 2. Render Jelly
            let stress = jelly.get_vertex_stress();

            for tri in &jelly.mesh.indices {
                let i0 = tri[0];
                let i1 = tri[1];
                let i2 = tri[2];

                let v0 = jelly.mesh.vertices[i0];
                let v1 = jelly.mesh.vertices[i1];
                let v2 = jelly.mesh.vertices[i2];

                let (c0, w0) = mvp.transform_point(v0);
                let (c1, w1) = mvp.transform_point(v1);
                let (c2, w2) = mvp.transform_point(v2);

                if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                    continue;
                }

                // Color based on stress
                let s = (stress[i0] + stress[i1] + stress[i2]) / 3.0;
                // Green -> Red gradient
                let t = (s * 10.0).clamp(0.0, 1.0);
                let r = (t * 255.0) as u32;
                let g = ((1.0 - t) * 255.0) as u32;
                let color = 0xFF000000 | (r << 16) | (g << 8);

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
    use abrash::mesh::Mesh;
    use abrash::platform::{
        SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
    };
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::zbuffer::ZBuffer;
    use abrash_render::experimental::jelly::SoftBody;
    use abrash_render::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene, render_sdf};
    use std::io;

    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        run_windowed(JellyApp::new().unwrap());
        Ok(())
    }

    struct JellyApp {
        width: u32,
        height: u32,
        framebuffer: Framebuffer,
        zbuffer: ZBuffer,
        presenter: Option<SoftwarePresenter>,
        sdf_scene: SdfScene,
        jelly: SoftBody,
    }

    impl JellyApp {
        fn new() -> Result<Self, io::Error> {
            let width = WIDTH;
            let height = HEIGHT;

            let framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            let zbuffer = ZBuffer::new(width, height).map_err(io::Error::other)?;

            let mut sdf_scene = SdfScene::new();
            sdf_scene.add(SdfObject {
                primitive: SdfPrimitive::Plane {
                    normal: Vec3::new(0.0, 1.0, 0.0),
                    distance: 2.0,
                },
                color: 0xFF55_5555,
            });
            sdf_scene.add(SdfObject {
                primitive: SdfPrimitive::Sphere {
                    radius: 1.5,
                    center: Vec3::new(0.0, -2.0, 0.0),
                },
                color: 0xFF00_00FF,
            });

            let mut mesh = Mesh::cube(2.0);
            for vertex in &mut mesh.vertices {
                vertex.y += 5.0;
            }

            let mut jelly = SoftBody::new(mesh, 1.0, 150.0, 2.0).map_err(io::Error::other)?;
            let diag_len = (jelly.mesh.vertices[0] - jelly.mesh.vertices[6]).length();
            jelly.add_spring(0, 6, diag_len).map_err(io::Error::other)?;
            jelly.add_spring(1, 7, diag_len).map_err(io::Error::other)?;
            jelly.add_spring(2, 4, diag_len).map_err(io::Error::other)?;
            jelly.add_spring(3, 5, diag_len).map_err(io::Error::other)?;

            Ok(Self {
                width,
                height,
                framebuffer,
                zbuffer,
                presenter: None,
                sdf_scene,
                jelly,
            })
        }

        fn rebuild_buffers(&mut self, width: u32, height: u32) -> Result<(), io::Error> {
            self.width = width;
            self.height = height;
            self.framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            self.zbuffer = ZBuffer::new(width, height).map_err(io::Error::other)?;
            Ok(())
        }

        fn render_scene(&mut self) {
            self.framebuffer.clear(0xFF10_1010);
            self.zbuffer.clear();

            let projection =
                Mat4::perspective(1.0, self.width as f32 / self.height as f32, 0.1, 100.0);
            let view = Mat4::look_at(
                Vec3::new(0.0, 2.0, 12.0),
                Vec3::new(0.0, -1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            let mvp = projection * view;
            let stress = self.jelly.get_vertex_stress();

            render_sdf(
                &mut self.framebuffer,
                &mut self.zbuffer,
                &self.sdf_scene,
                &view,
                &projection,
                Vec3::new(0.0, 2.0, 12.0),
            );

            for tri in &self.jelly.mesh.indices {
                let i0 = tri[0];
                let i1 = tri[1];
                let i2 = tri[2];

                let v0 = self.jelly.mesh.vertices[i0];
                let v1 = self.jelly.mesh.vertices[i1];
                let v2 = self.jelly.mesh.vertices[i2];

                let (c0, w0) = mvp.transform_point(v0);
                let (c1, w1) = mvp.transform_point(v1);
                let (c2, w2) = mvp.transform_point(v2);

                if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                    continue;
                }

                let s = (stress[i0] + stress[i1] + stress[i2]) / 3.0;
                let t = (s * 10.0).clamp(0.0, 1.0);
                let r = (t * 255.0) as u32;
                let g = ((1.0 - t) * 255.0) as u32;
                let color = 0xFF00_0000 | (r << 16) | (g << 8);

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

    impl WindowApp for JellyApp {
        type Error = io::Error;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: "Jelly Physics Demo".to_string(),
                width: self.width,
                height: self.height,
                vsync: true,
            }
        }

        fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
            self.presenter = Some(SoftwarePresenter::new(ctx.window.clone()).map_err(io::Error::other)?);
            Ok(())
        }

        fn resize(
            &mut self,
            _ctx: &WindowContext<'_>,
            width: u32,
            height: u32,
        ) -> Result<(), Self::Error> {
            self.rebuild_buffers(width, height)
        }

        fn update(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
            let dt = ctx.dt_seconds.min(0.05);
            for _ in 0..4 {
                self.jelly.update(dt / 4.0);
                self.jelly.collide_sdf(&self.sdf_scene, 0.7);
            }
            Ok(())
        }

        fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
            self.render_scene();
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
    println!("\n{}", "🍮 Jelly Physics Demo".bold().magenta());
    println!("{}", "=====================".dark_grey());

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
            Cell::new("Collision"),
            Cell::new("Signed Distance Fields (SDF)").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Rendering"),
            Cell::new("Software Rasterizer + Stress Visualization").fg(Color::Blue),
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
                Cell::new("Try running with:\ncargo run --example jelly_demo --features nova")
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
                Cell::new("Try running with:\ncargo run --example jelly_demo --features nova")
                    .fg(Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
