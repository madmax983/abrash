use abrash::ascii::AsciiCharset;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::obj_loader::load_obj;
#[cfg(all(not(feature = "backend-winit"), not(feature = "backend-tui")))]
use abrash::platform::Window;
use abrash::rasterizer::fill_triangle_3d;
#[cfg(all(not(feature = "backend-winit"), not(feature = "backend-tui")))]
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;

use crossterm::style::Stylize;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color as TuiColor, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

#[cfg_attr(feature = "backend-tui", allow(dead_code))]
const WIDTH: u32 = 800;
#[cfg_attr(feature = "backend-tui", allow(dead_code))]
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

// Embed a simple spaceship-like OBJ
const SPACESHIP_OBJ: &str = r"
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
";

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

fn render_mesh(
    framebuffer: &mut Framebuffer,
    zbuffer: &mut ZBuffer,
    mesh: &Mesh,
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
        let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

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

struct AsciiWidget<'a> {
    framebuffer: &'a Framebuffer,
    charset: AsciiCharset,
    colored: bool,
}

const fn pixel_luminance(pixel: u32) -> u8 {
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;
    ((77 * r + 150 * g + 29 * b) >> 8) as u8
}

impl Widget for AsciiWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let term_w = area.width as usize;
        let term_h = area.height as usize;
        let fb_w = self.framebuffer.width() as usize;
        let fb_h = self.framebuffer.height() as usize;

        for y in 0..term_h {
            for x in 0..term_w {
                // Nearest neighbor sampling
                let fb_x = (x * fb_w) / term_w;
                let fb_y = (y * fb_h) / term_h;

                if fb_x >= fb_w || fb_y >= fb_h {
                    continue;
                }

                if let Some(pixel) = self.framebuffer.get_pixel(fb_x as i32, fb_y as i32) {
                    let luminance = pixel_luminance(pixel);
                    let ch = self.charset.map(luminance);

                    let cell = buf.cell_mut((area.x + x as u16, area.y + y as u16));
                    if let Some(cell) = cell {
                        cell.set_char(ch);
                        if self.colored {
                            let r = ((pixel >> 16) & 0xFF) as u8;
                            let g = ((pixel >> 8) & 0xFF) as u8;
                            let b = (pixel & 0xFF) as u8;
                            cell.set_fg(TuiColor::Rgb(r, g, b));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "backend-winit")]
mod winit_demo {
    use super::{
        BACKGROUND, Framebuffer, HEIGHT, Mat4, Mesh, PI, Vec3, WIDTH, ZBuffer, render_mesh,
    };
    use abrash::platform::{
        SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
    };
    use std::io;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run(
        mesh: Mesh,
        normals: Vec<Vec3>,
        source_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        run_windowed(ObjViewerApp::new(mesh, normals, source_name).unwrap());
        Ok(())
    }

    struct ObjViewerApp {
        width: u32,
        height: u32,
        framebuffer: Framebuffer,
        zbuffer: ZBuffer,
        presenter: Option<SoftwarePresenter>,
        mesh: Mesh,
        normals: Vec<Vec3>,
        base_color: Vec3,
        angle_y: f32,
        source_name: String,
    }

    impl ObjViewerApp {
        fn new(mesh: Mesh, normals: Vec<Vec3>, source_name: String) -> Result<Self, io::Error> {
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
                mesh,
                normals,
                base_color: Vec3::new(0.4, 0.6, 1.0),
                angle_y: 0.0,
                source_name,
            })
        }

        fn rebuild_buffers(&mut self, width: u32, height: u32) -> Result<(), io::Error> {
            self.width = width;
            self.height = height;
            self.framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            self.zbuffer = ZBuffer::new(width, height).map_err(io::Error::other)?;
            Ok(())
        }

        fn render_frame(&mut self) {
            self.framebuffer.clear(BACKGROUND);
            self.zbuffer.clear();

            let projection =
                Mat4::perspective(PI / 3.0, self.width as f32 / self.height as f32, 0.1, 100.0);
            let view = Mat4::look_at(
                Vec3::new(0.0, 1.0, 3.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            let model = Mat4::rotation_y(self.angle_y);
            let mvp = projection * (view * model);
            let normal_mat = model;

            render_mesh(
                &mut self.framebuffer,
                &mut self.zbuffer,
                &self.mesh,
                &self.normals,
                &mvp,
                &normal_mat,
                self.base_color,
            );
        }
    }

    impl WindowApp for ObjViewerApp {
        type Error = io::Error;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: format!("Abrash - OBJ Viewer - {}", self.source_name),
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
            self.angle_y += ctx.dt_seconds;
            Ok(())
        }

        fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            self.render_frame();
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


#[allow(clippy::missing_const_for_fn)]
fn print_banner() {
    println!("\n{}", "🎨 Abrash OBJ Viewer".bold().cyan());
    println!("{}", "=====================".dark_grey());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let args = Args::parse();



    let (mesh_source, source_name) = args.input.map_or_else(
        || (SPACESHIP_OBJ.to_string(), "Built-in Spaceship".to_string()),
        |path| {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    let mut error_table = Table::new();
                    error_table
                        .load_preset(presets::UTF8_FULL)
                        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                        .set_header(vec![
                            Cell::new("❌ Error Reading File")
                                .add_attribute(comfy_table::Attribute::Bold)
                                .fg(Color::Red),
                        ])
                        .add_row(vec![
                            Cell::new(format!("File '{}': {}", path.display(), e))
                                .fg(Color::Yellow),
                        ]);
                    eprintln!("\n{error_table}");
                    std::process::exit(1);
                }
            };
            (content, path.display().to_string())
        },
    );

    // Load the mesh
    let mesh = match load_obj(&mesh_source) {
        Ok(m) => {
            let mut table = Table::new();
            table
                .load_preset(presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
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

            println!("\n{}", "📦 Asset Information".bold());
            println!("{table}");
            m
        }
        Err(e) => {
            let mut error_table = Table::new();
            error_table
                .load_preset(presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_header(vec![
                    Cell::new("❌ Error Parsing OBJ")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(Color::Red),
                ])
                .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    };

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
            Cell::new("(Coming Soon)").fg(Color::DarkGrey),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\n{}", "🎮 Controls".bold());
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

        // TUI Setup
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut frame_count = 0;
        let start_time = std::time::Instant::now();

        loop {
            // Event Handling
            if event::poll(std::time::Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }

            angle_y += 0.02;

            framebuffer.clear(BACKGROUND);
            zbuffer.clear();

            let model = Mat4::rotation_y(angle_y);
            let mvp = projection * (view * model);
            let normal_mat = model;

            render_mesh(
                &mut framebuffer,
                &mut zbuffer,
                &mesh,
                &normals,
                &mvp,
                &normal_mat,
                base_color,
            );

            frame_count += 1;
            let elapsed = start_time.elapsed().as_secs_f32();
            let fps = frame_count as f32 / elapsed.max(0.001);

            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(f.area());

                let ascii_widget = AsciiWidget {
                    framebuffer: &framebuffer,
                    charset: AsciiCharset::Standard,
                    colored: args.colored_ascii,
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .title(format!(" Abrash OBJ Viewer: {source_name} "))
                    .title_style(Style::default().fg(TuiColor::Cyan));

                f.render_widget(ascii_widget, block.inner(chunks[0]));
                f.render_widget(block, chunks[0]);

                let status = format!(
                    " FPS: {:.1} | Verts: {} | Tris: {} | [Q] Quit ",
                    fps,
                    mesh.vertices.len(),
                    mesh.indices.len()
                );
                let status_bar = Paragraph::new(status)
                    .style(Style::default().fg(TuiColor::Black).bg(TuiColor::Cyan));
                f.render_widget(status_bar, chunks[1]);
            })?;
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    } else {
        #[cfg(feature = "backend-winit")]
        {
            return winit_demo::run(mesh, normals, source_name);
        }

        #[cfg(all(not(feature = "backend-winit"), not(feature = "backend-tui")))]
        {
            let window_title = format!("Abrash - OBJ Viewer - {source_name}");
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

                render_mesh(
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

        #[cfg(all(not(feature = "backend-winit"), feature = "backend-tui"))]
        {
            eprintln!(
                "Graphical window mode is unavailable with backend-tui. Use --ascii or --colored-ascii."
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
