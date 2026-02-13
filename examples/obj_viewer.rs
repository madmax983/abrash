use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::obj_loader::load_obj;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color as TableColor, Table, presets};
use std::f32::consts::PI;
use std::fmt::{self, Write};
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

    /// Render in ASCII mode to stdout
    #[arg(long)]
    ascii: bool,

    /// Render in Colored ASCII mode to stdout
    #[arg(long)]
    colored_ascii: bool,

    /// Width for ASCII rendering (default: 100)
    #[arg(long, default_value = "100")]
    width: u32,

    /// Height for ASCII rendering (default: 50)
    #[arg(long, default_value = "50")]
    height: u32,
}

// --- ASCII Converter (Moved from experimental) ---

/// Character set used for luminance mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiCharset {
    /// Standard ASCII gradient: ` .:-=+*#%@`
    Standard,
    /// Block characters: ` ░▒▓█`
    Blocks,
    /// Minimal set: ` .:`
    Minimal,
    /// Binary set: ` 1`
    Binary,
}

impl AsciiCharset {
    /// Returns the characters in the set, ordered from darkest to brightest.
    const fn chars(self) -> &'static [char] {
        match self {
            Self::Standard => &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'],
            Self::Blocks => &[' ', '░', '▒', '▓', '█'],
            Self::Minimal => &[' ', '.', ':'],
            Self::Binary => &[' ', '1'],
        }
    }

    /// Maps a luminance value (0-255) to a character in the set.
    fn map(self, luminance: u8) -> char {
        let chars = self.chars();
        let len = chars.len();
        // Calculate index: (luminance * len) / 256
        // Use u16 to prevent overflow before division
        let index = (u16::from(luminance) * len as u16) >> 8;
        chars[index.min((len - 1) as u16) as usize]
    }
}

/// Converter for rendering a [`Framebuffer`] as ASCII art.
pub struct AsciiConverter<'a> {
    framebuffer: &'a Framebuffer,
    charset: AsciiCharset,
}

impl<'a> AsciiConverter<'a> {
    /// Creates a new ASCII converter for the given framebuffer.
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer, charset: AsciiCharset) -> Self {
        Self {
            framebuffer,
            charset,
        }
    }

    /// Calculates luminance using standard weights (Rec. 601).
    /// Y = 0.299*R + 0.587*G + 0.114*B
    const fn pixel_luminance(pixel: u32) -> u8 {
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        // Fixed-point calculation: (77*R + 150*G + 29*B) >> 8
        ((77 * r + 150 * g + 29 * b) >> 8) as u8
    }

    /// Converts the framebuffer to a string with ANSI color codes.
    #[must_use]
    pub fn to_colored_string(&self) -> String {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let mut result = String::with_capacity(((width * 20) * height) as usize);

        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = self.framebuffer.get_pixel(x as i32, y as i32) {
                    let luminance = Self::pixel_luminance(pixel);
                    let ch = self.charset.map(luminance);

                    let r = (pixel >> 16) & 0xFF;
                    let g = (pixel >> 8) & 0xFF;
                    let b = pixel & 0xFF;

                    let _ = write!(result, "\x1b[38;2;{r};{g};{b}m{ch}");
                } else {
                    result.push(' ');
                }
            }
            result.push_str("\x1b[0m\n");
        }
        result
    }
}

impl fmt::Display for AsciiConverter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let mut result = String::with_capacity(((width + 1) * height) as usize);

        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = self.framebuffer.get_pixel(x as i32, y as i32) {
                    let luminance = Self::pixel_luminance(pixel);
                    result.push(self.charset.map(luminance));
                }
            }
            result.push('\n');
        }
        write!(f, "{result}")
    }
}

// --- Main ---

fn render_mesh_to_framebuffer(
    framebuffer: &mut Framebuffer,
    zbuffer: &mut ZBuffer,
    mesh: &Mesh,
    // Normals are computed per face in main
    normals: &[Vec3],
    mvp: &Mat4,
    normal_mat: &Mat4,
    base_color: Vec3,
) {
     for (i, tri_indices) in mesh.indices.iter().enumerate() {
        let v0 = mesh.vertices[tri_indices[0]];
        let v1 = mesh.vertices[tri_indices[1]];
        let v2 = mesh.vertices[tri_indices[2]];

        // Transform vertices
        let (clip0, w0) = mvp.transform_point(v0);
        let (clip1, w1) = mvp.transform_point(v1);
        let (clip2, w2) = mvp.transform_point(v2);

        // Simple backface culling
        // Note: this is clip space w, which is basically -z in view space.
        // If w < 0, it's behind camera. Simple cull.
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
        let diffuse = world_normal.dot(light_dir).max(0.5);

        let color_vec = base_color * diffuse;

        // Pack color (ARGB)
        let r = (color_vec.x * 255.0).min(255.0) as u32;
        let g = (color_vec.y * 255.0).min(255.0) as u32;
        let b = (color_vec.z * 255.0).min(255.0) as u32;
        let color = 0xFF000000 | (r << 16) | (g << 8) | b;

        fill_triangle_3d(
            framebuffer,
            zbuffer,
            (clip0, w0),
            (clip1, w1),
            (clip2, w2),
            color,
        );
    }
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
                    Cell::new("Property").fg(TableColor::Cyan),
                    Cell::new("Value").fg(TableColor::Cyan),
                ])
                .add_row(vec![
                    Cell::new("Source"),
                    Cell::new(&source_name).fg(TableColor::Yellow),
                ])
                .add_row(vec![
                    Cell::new("Status"),
                    Cell::new("✅ Loaded Successfully").fg(TableColor::Green),
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
            Cell::new("Input").fg(TableColor::Cyan),
            Cell::new("Action").fg(TableColor::Cyan),
        ])
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("(Coming Soon)").fg(TableColor::DarkGrey),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\n🎮 Controls");
    println!("{controls}\n");

    // Compute normals for flat shading logic
    let normals = mesh.compute_face_normals();
    let base_color = Vec3::new(0.4, 0.6, 1.0); // Light blue
    let mut angle_y: f32 = 0.0;

    if args.ascii || args.colored_ascii {
        let width = args.width;
        let height = args.height;
        let mut framebuffer = Framebuffer::new(width, height)?;
        let mut zbuffer = ZBuffer::new(width, height)?;

        let projection = Mat4::perspective(PI / 3.0, width as f32 / height as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        // Clear screen
        print!("\x1b[2J");

        // Simple loop for ASCII animation (runs for a few seconds then exits or loops forever? Example code looped forever)
        loop {
            angle_y += 0.02;

            framebuffer.clear(BACKGROUND);
            zbuffer.clear();

            let model = Mat4::rotation_y(angle_y);
            let mvp = projection * (view * model);
            let normal_mat = model;

            render_mesh_to_framebuffer(
                &mut framebuffer,
                &mut zbuffer,
                &mesh,
                &normals,
                &mvp,
                &normal_mat,
                base_color,
            );

            let converter = AsciiConverter::new(&framebuffer, AsciiCharset::Standard);
            let output = if args.colored_ascii {
                converter.to_colored_string()
            } else {
                converter.to_string()
            };

            print!("\x1b[H{}", output);
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    } else {
        let window_title = format!("Abrash - OBJ Viewer - {}", source_name);
        let mut window = Window::new(&window_title, WIDTH, HEIGHT)?;
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
        let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
        let mut timestep = FixedTimestep::new(60);

        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        while window.is_open() {
            window.poll_events();

            let steps = timestep.update();
            for _ in 0..steps {
                angle_y += 1.0 * timestep.dt();
            }

            framebuffer.clear(BACKGROUND);
            zbuffer.clear();

            let model = Mat4::rotation_y(angle_y);
            let mvp = projection * (view * model);
            let normal_mat = model;

            render_mesh_to_framebuffer(
                &mut framebuffer,
                &mut zbuffer,
                &mesh,
                &normals,
                &mvp,
                &normal_mat,
                base_color,
            );

            window.blit_framebuffer(&framebuffer);
        }
    }

    Ok(())
}
