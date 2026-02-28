use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::error::Error;

#[cfg(feature = "nova")]
mod demo {
    use abrash::experimental::jelly::SoftBody;
    use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene, render_sdf};
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::mesh::Mesh;
    use abrash::platform::{Window, WindowBackend};
    use abrash::rasterizer::fill_triangle_3d;
    use abrash::zbuffer::ZBuffer;
    use std::time::Instant;

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
        jelly.spring_indices_a.push(0);
        jelly.spring_indices_b.push(6);
        jelly.spring_rest_lengths.push(diag_len);
        jelly.spring_indices_a.push(1);
        jelly.spring_indices_b.push(7);
        jelly.spring_rest_lengths.push(diag_len);
        jelly.spring_indices_a.push(2);
        jelly.spring_indices_b.push(4);
        jelly.spring_rest_lengths.push(diag_len);
        jelly.spring_indices_a.push(3);
        jelly.spring_indices_b.push(5);
        jelly.spring_rest_lengths.push(diag_len);

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
                let color = 0xFF000000 | (r << 16) | (g << 8) | 0x00;

                fill_triangle_3d(&mut fb, &mut zb, (c0, w0), (c1, w1), (c2, w2), color);
            }

            window.blit_framebuffer(&fb);
        }
        Ok(())
    }
}

fn print_banner() {
    println!("\n{}", "🍮 Jelly Physics Demo".bold().magenta());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
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

fn main() -> Result<(), Box<dyn Error>> {
    print_banner();

    #[cfg(feature = "nova")]
    {
        demo::run()
    }
    #[cfg(not(feature = "nova"))]
    {
        println!("\n{}", "⚠️  Missing Feature: Nova".bold().red());
        println!(
            "{}",
            "This demo requires the 'nova' feature to run.".white()
        );
        println!("\nTry running with:");
        println!(
            "{}",
            "cargo run --example jelly_demo --features nova".green()
        );
        Ok(())
    }
}
