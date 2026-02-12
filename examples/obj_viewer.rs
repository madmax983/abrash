use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::obj_loader::load_obj;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF101010;

// Embed a simple spaceship-like OBJ
const SPACESHIP_OBJ: &str = r#"
# Simple Spacerocket
v 0.0 1.5 0.0
v 0.5 -0.5 0.5
v -0.5 -0.5 0.5
v -0.5 -0.5 -0.5
v 0.5 -0.5 -0.5
v 0.0 -0.8 0.0
# Top pyramid
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
# Bottom inverted pyramid (engine)
f 6 3 2
f 6 4 3
f 6 5 4
f 6 2 5
"#;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Simple software-rendered OBJ viewer for Abrash Engine",
    long_about = None
)]
struct Args {
    /// Path to the OBJ file to load (optional)
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("\n🎨 Abrash OBJ Viewer");

    let (mesh_source, source_name) = match args.input {
        Some(path) => {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to read file '{}': {}", path.display(), e);
                    std::process::exit(1);
                }
            };
            (content, path.display().to_string())
        }
        None => (SPACESHIP_OBJ.to_string(), "Built-in Spaceship".to_string()),
    };

    // Load the mesh
    let mesh = match load_obj(&mesh_source) {
        Ok(m) => {
            let mut table = Table::new();
            table
                .load_preset(presets::UTF8_FULL)
                .set_header(vec![
                    Cell::new("Property").fg(Color::Cyan),
                    Cell::new("Value").fg(Color::Cyan),
                ])
                .add_row(vec![
                    Cell::new("Source"),
                    Cell::new(&source_name).fg(Color::Yellow),
                ])
                .add_row(vec![
                    Cell::new("Status"),
                    Cell::new("✅ Loaded Successfully").fg(Color::Green),
                ])
                .add_row(vec![
                    Cell::new("Vertices"),
                    Cell::new(m.vertices.len().to_string()),
                ])
                .add_row(vec![
                    Cell::new("Triangles"),
                    Cell::new(m.indices.len().to_string()),
                ]);

            println!("{table}");
            m
        }
        Err(e) => {
            eprintln!("❌ Failed to parse OBJ: {}", e);
            std::process::exit(1);
        }
    };

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("(Coming Soon)").fg(Color::DarkGrey),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\n🎮 Controls");
    println!("{controls}\n");

    let window_title = format!("Abrash - OBJ Viewer - {}", source_name);
    let mut window = Window::new(&window_title, WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Compute normals for flat shading logic (simple color variation)
    let normals = mesh.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 1.0, 3.0), // eye
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );

    let mut angle_y: f32 = 0.0;

    // Simple palette based on normal direction
    let base_color = Vec3::new(0.4, 0.6, 1.0); // Light blue

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 1.0 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Model matrix (rotation)
        let model = Mat4::rotation_y(angle_y);

        // MVP matrix
        let mvp = projection * (view * model);

        // Rotation matrix for normals (upper 3x3 of model)
        let normal_mat = model; // For rotation only, this is fine

        // Transform and render each triangle
        for (i, tri_indices) in mesh.indices.iter().enumerate() {
            let v0 = mesh.vertices[tri_indices[0]];
            let v1 = mesh.vertices[tri_indices[1]];
            let v2 = mesh.vertices[tri_indices[2]];

            // Transform vertices
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple backface culling
            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // Calculate color based on normal
            // Since Mesh doesn't store normals per vertex, we use the face normal
            let normal = if i < normals.len() {
                normals[i]
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };

            // Rotate normal
            let world_normal = normal_mat.transform_normal(normal);

            // Simple directional light from top-right
            let light_dir = Vec3::new(0.5, 1.0, 0.5).normalize();
            let diffuse = world_normal.dot(light_dir).max(0.2);

            let color_vec = base_color * diffuse;

            // Pack color (ARGB)
            let r = (color_vec.x * 255.0).min(255.0) as u32;
            let g = (color_vec.y * 255.0).min(255.0) as u32;
            let b = (color_vec.z * 255.0).min(255.0) as u32;
            let color = 0xFF000000 | (r << 16) | (g << 8) | b;

            fill_triangle_3d(
                &mut framebuffer,
                &mut zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                color,
            );
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
