//! Abrash Engine - CLI Dashboard & Launcher
//!
//! Provides a TUI (Text User Interface) to explore and launch demos.
//!
//! This module uses `ratatui` for rendering the interface and `crossterm` for input handling.
//! It scans the available demos defined in `DEMOS` constant and allows the user to
//! select and launch them via `cargo run`.

use clap::Parser;
use comfy_table::{
    Cell as ComfyCell, Color as ComfyColor, Table as ComfyTable, presets as ComfyPresets,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    style::Stylize,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table},
};
use std::{error::Error, io, process::Command};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the demo to run directly
    #[arg(long, short)]
    demo: Option<String>,

    /// List all available demos
    #[arg(long, short)]
    list: bool,

    /// Run the interactive TUI dashboard
    #[arg(long)]
    tui: bool,
}

#[derive(Debug, Clone, Copy)]
enum DemoCategory {
    Cpu3D,
    Gpu3D,
    Simulation,
    Utility,
}

impl DemoCategory {
    const fn icon(self) -> char {
        match self {
            Self::Cpu3D => '🧊',
            Self::Gpu3D => '🚀',
            Self::Simulation => '✨',
            Self::Utility => '🔧',
        }
    }
}

struct Demo {
    name: &'static str,
    category: DemoCategory,
    description: &'static str,
    instructions: &'static str,
    example_name: &'static str,
}

const DEMOS: &[Demo] = &[
    Demo {
        name: "Lit Cube",
        category: DemoCategory::Cpu3D,
        description: "Flat shaded cube with directional lighting",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "lit_cube",
    },
    Demo {
        name: "Cube 3D",
        category: DemoCategory::Cpu3D,
        description: "Basic 3D cube rendering",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "cube_3d",
    },
    Demo {
        name: "Kinect Depth Filter",
        category: DemoCategory::PostProcess,
        description: "Simulates Kinect/structured light depth sensor",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "kinect_depth_demo",
    },
    Demo {
        name: "OBJ Viewer",
        category: DemoCategory::Utility,
        description: "Loads and renders a 3D model (Spaceship)",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "obj_viewer",
    },
    Demo {
        name: "GPU Cube",
        category: DemoCategory::Gpu3D,
        description: "Hardware-accelerated cube rendering with wgpu",
        instructions: "Mouse: Drag to orbit, wheel to zoom\nKeyboard: Arrows/WASD orbit, Q/E zoom, Space toggle auto-rotate, R reset",
        example_name: "gpu_cube",
    },
    Demo {
        name: "GPU Pyramid",
        category: DemoCategory::Gpu3D,
        description: "Hardware-accelerated pyramid mesh via generic GPU mesh runner",
        instructions: "Mouse: Drag to orbit, wheel to zoom\nKeyboard: Arrows/WASD orbit, Q/E zoom, Space toggle auto-rotate, R reset",
        example_name: "gpu_pyramid",
    },
    Demo {
        name: "GPU OBJ",
        category: DemoCategory::Gpu3D,
        description: "Hardware-accelerated OBJ rendering (Spaceship)",
        instructions: "Mouse: Drag to orbit, wheel to zoom\nKeyboard: Arrows/WASD orbit, Q/E zoom, Space toggle auto-rotate, R reset",
        example_name: "gpu_obj",
    },
    Demo {
        name: "Particles",
        category: DemoCategory::Simulation,
        description: "Interactive particle system simulation",
        instructions: "Mouse: None\nKeyboard: Auto-rotating",
        example_name: "particles",
    },
    Demo {
        name: "Vision Demo",
        category: DemoCategory::Simulation,
        description: "Simulates thermal, sonar, and night vision effects",
        instructions: "Mouse: None\nKeyboard: Auto-rotating",
        example_name: "vision_demo",
    },
    Demo {
        name: "Normal Mapping",
        category: DemoCategory::Cpu3D,
        description: "Per-pixel lighting with normal maps",
        instructions: "Mouse: None\nKeyboard: Auto-rotating light",
        example_name: "normal_mapping_demo",
    },
    Demo {
        name: "Skybox",
        category: DemoCategory::Cpu3D,
        description: "Renders a cubemap skybox",
        instructions: "Mouse: None\nKeyboard: Auto-rotating camera",
        example_name: "skybox_demo",
    },
    Demo {
        name: "Raytracer",
        category: DemoCategory::Simulation,
        description: "Experimental CPU raytracer (Reflections/Shadows)",
        instructions: "Mouse: None\nKeyboard: Auto-rotating scene",
        example_name: "raytracer_demo",
    },
    Demo {
        name: "Cloth Simulation",
        category: DemoCategory::Simulation,
        description: "Soft-body physics cloth simulation",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "cloth_demo",
    },
    Demo {
        name: "Pixel Sort",
        category: DemoCategory::Simulation,
        description: "Post-processing pixel sorting effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "pixel_sort_demo",
    },
    Demo {
        name: "Directional Blur",
        category: DemoCategory::Simulation,
        description: "Post-processing directional blur effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "directional_blur_demo",
    },
    Demo {
        name: "God Rays",
        category: DemoCategory::Simulation,
        description: "Post-processing god rays (volumetric light) effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "god_rays_demo",
    },
    Demo {
        name: "Selective Color Demo",
        category: DemoCategory::Simulation,
        description: "Converts image to grayscale except for a target hue.",
        instructions: "Observe the rotating cubes. Only the targeted hue will remain colored.",
        example_name: "selective_color_demo",
    },
    Demo {
        name: "Jelly Physics",
        category: DemoCategory::Simulation,
        description: "Soft-body jelly physics simulation",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "jelly_demo",
    },
    Demo {
        name: "Color Splash",
        category: DemoCategory::Utility,
        description: "Selective color post-processing filter",
        instructions: "Mouse: None\nKeyboard: Auto-rotating",
        example_name: "color_splash_demo",
    },
    Demo {
        name: "Anaglyph 3D",
        category: DemoCategory::Simulation,
        description: "Post-processing anaglyph 3D effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "anaglyph_demo",
    },
    Demo {
        name: "Chromatic Aberration",
        category: DemoCategory::Simulation,
        description: "Post-processing chromatic aberration effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "chromatic_aberration_demo",
    },
    Demo {
        name: "Voronoi Diagram",
        category: DemoCategory::Simulation,
        description: "Post-processing Voronoi effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "voronoi_demo",
    },
    Demo {
        name: "Vignette Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing vignette effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "vignette_demo",
    },
    Demo {
        name: "Swirl Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing swirl distortion effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "swirl_demo",
    },
    Demo {
        name: "Wobble Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing wobble distortion effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "wobble_demo",
    },
    Demo {
        name: "Fisheye Lens",
        category: DemoCategory::Simulation,
        description: "Post-processing ultra-wide barrel distortion effect",
        instructions: "Mouse: None\nKeyboard: Auto-rotating",
        example_name: "fisheye_demo",
    },
    Demo {
        name: "Tilt Shift",
        category: DemoCategory::Simulation,
        description: "Post-processing tilt shift depth-of-field effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "tilt_shift_demo",
    },
    Demo {
        name: "Posterize Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing posterization effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "posterize_demo",
    },
    Demo {
        name: "Emboss Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing emboss effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "emboss_demo",
    },
    Demo {
        name: "Glitch Effect",
        category: DemoCategory::Simulation,
        description: "Post-processing digital corruption and RGB separation",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "glitch_demo",
    },
    Demo {
        name: "Edge Glow",
        category: DemoCategory::Simulation,
        description: "Post-processing edge detection and glow effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "edge_glow_demo",
    },
    Demo {
        name: "Reflection Demo",
        category: DemoCategory::Cpu3D,
        description: "3D scene demonstrating planar reflections",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "reflection_demo",
    },
    Demo {
        name: "GPU MVP Cube",
        category: DemoCategory::Gpu3D,
        description: "Hardware-accelerated cube with Model-View-Projection matrix",
        instructions: "Mouse: Drag to orbit, wheel to zoom
Keyboard: Arrows/WASD orbit, Q/E zoom, Space toggle auto-rotate, R reset",
        example_name: "gpu_mvp_cube",
    },
    Demo {
        name: "Halftone Filter",
        category: DemoCategory::Simulation,
        description: "Post-processing halftone effect",
        instructions: "Mouse: None
Keyboard: Auto-rotating",
        example_name: "halftone_demo",
    },
    Demo {
        name: "Arboretum",
        category: DemoCategory::Simulation,
        description: "Procedural L-System plant generator",
        instructions: "Mouse: None
Keyboard: Interactive",
        example_name: "arboretum_cli",
    },
    Demo {
        name: "Screen Melt",
        category: DemoCategory::Simulation,
        description: "Post-processing classic screen melt effect",
        instructions: "Mouse: None\nKeyboard: None",
        example_name: "melt_demo",
    },
    Demo {
        name: "Digital Rain",
        category: DemoCategory::Simulation,
        description: "Post-processing falling characters (Matrix style) effect",
        instructions: "Mouse: None\nKeyboard: None",
        example_name: "digital_rain_demo",
    },
    Demo {
        name: "Frosted Glass",
        category: DemoCategory::Simulation,
        description: "Post-processing textured privacy glass effect",
        instructions: "Mouse: None\nKeyboard: None",
        example_name: "frosted_glass_demo",
    },
    Demo {
        name: "Steganography",
        category: DemoCategory::Simulation,
        description: "Encodes and decodes a hidden message in a Plasma framebuffer",
        instructions: "Mouse: None\nKeyboard: Close window to exit",
        example_name: "steganography_demo",
    },
    Demo {
        name: "Reaction-Diffusion",
        category: DemoCategory::Simulation,
        description: "Simulates Gray-Scott Turing patterns over time",
        instructions: "Mouse: Click to add drops\nKeyboard: Space to clear, C to reset",
        example_name: "reaction_diffusion_demo",
    },
    Demo {
        name: "Fractal Explorer",
        category: DemoCategory::Simulation,
        description: "Mandelbrot set generator.",
        instructions: "Mouse: None\nKeyboard: ESC to exit",
        example_name: "fractal_demo",
    },
];

fn is_gpu_render_example(example_name: &str) -> bool {
    example_name.starts_with("gpu_")
}

fn is_nova_example(example_name: &str) -> bool {
    example_name == "cloth_demo"
        || example_name == "raytracer_demo"
        || example_name == "vision_demo"
        || example_name == "pixel_sort_demo"
        || example_name == "directional_blur_demo"
        || example_name == "god_rays_demo"
        || example_name == "jelly_demo"
        || example_name == "color_splash_demo"
        || example_name == "anaglyph_demo"
        || example_name == "fractal_demo"
        || example_name == "chromatic_aberration_demo"
        || example_name == "voronoi_demo"
        || example_name == "vignette_demo"
        || example_name == "kinect_depth_demo"
        || example_name == "swirl_demo"
        || example_name == "wobble_demo"
        || example_name == "fisheye_demo"
        || example_name == "tilt_shift_demo"
        || example_name == "posterize_demo"
        || example_name == "emboss_demo"
        || example_name == "edge_glow_demo"
        || example_name == "glitch_demo"
        || example_name == "halftone_demo"
        || example_name == "arboretum_cli"
        || example_name == "melt_demo"
        || example_name == "digital_rain_demo"
        || example_name == "frosted_glass_demo"
        || example_name == "steganography_demo"
        || example_name == "reaction_diffusion_demo"
}

fn build_demo_command_args(example_name: &str, use_tui_backend: bool) -> Vec<String> {
    let mut args = vec![
        "cargo".to_string(),
        "run".to_string(),
        "--release".to_string(),
        "--example".to_string(),
        example_name.to_string(),
    ];

    if is_gpu_render_example(example_name) {
        args.push("--features".to_string());
        args.push("gpu-render".to_string());
        return args;
    }

    // ⚡ Bolt: Pre-allocate capacity for up to 2 features to avoid dynamic heap reallocations during push.
    let mut features = Vec::with_capacity(2);
    if use_tui_backend {
        args.push("--no-default-features".to_string());
        features.push("backend-tui");
    }
    if is_nova_example(example_name) {
        features.push("nova");
    }

    if !features.is_empty() {
        args.push("--features".to_string());
        args.push(features.join(","));
    }

    args
}

fn demo_command(example_name: &str, use_tui_backend: bool) -> String {
    build_demo_command_args(example_name, use_tui_backend).join(" ")
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if let Some(demo_name) = args.demo {
        run_demo(&demo_name, false)?;
        return Ok(());
    }

    if args.list {
        print_demo_list();
        return Ok(());
    }

    if args.tui {
        return run_tui_dashboard();
    }

    print_demo_list();
    Ok(())
}

fn run_tui_dashboard() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        // We can't easily access the Terminal instance here, but raw crossterm commands work
        original_hook(panic_info);
    }));

    let res = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("\n{}", "👋 Thanks for using Abrash Engine!".bold().cyan());

    if let Err(err) = res {
        let mut error_table = ComfyTable::new();
        error_table
            .load_preset(ComfyPresets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                ComfyCell::new("❌ Application Error")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(ComfyColor::Red),
            ])
            .add_row(vec![
                ComfyCell::new(format!("{err}")).fg(ComfyColor::Yellow),
            ]);

        println!("\n{error_table}");
    }

    Ok(())
}

fn print_demo_list() {
    let mut table = ComfyTable::new();
    table
        .load_preset(ComfyPresets::UTF8_BORDERS_ONLY)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            ComfyCell::new("Icon")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Magenta),
            ComfyCell::new("Name")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Magenta),
            ComfyCell::new("Category")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Magenta),
            ComfyCell::new("Description")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Magenta),
            ComfyCell::new("Command")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Magenta),
        ]);

    for demo in DEMOS {
        let row = vec![
            ComfyCell::new(demo.category.icon()),
            ComfyCell::new(demo.name).fg(ComfyColor::Cyan),
            ComfyCell::new(format!("{:?}", demo.category)).fg(ComfyColor::Yellow),
            ComfyCell::new(demo.description),
            ComfyCell::new(format!("abrash --demo {}", demo.example_name)).fg(ComfyColor::DarkGrey),
        ];

        table.add_row(row);
    }

    println!("{table}");
}

/// Application state for the TUI dashboard.
///
/// Tracks the currently selected demo in the list.
struct App {
    state: ListState,
}

impl App {
    fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
    }

    const fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= DEMOS.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    const fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    DEMOS.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

/// Runs the main event loop of the TUI dashboard.
///
/// Handles rendering the UI and processing keyboard input.
/// Returns `Ok(())` when the user quits, or an error if something fails.
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| render_ui(f, &mut app))?;

        #[allow(clippy::collapsible_if)]
        if let Event::Key(key) = event::read()? {
            if handle_input(key, &mut app, terminal)? {
                return Ok(());
            }
        }
    }
}

fn render_ui(f: &mut ratatui::Frame, app: &mut App) {
    // Main vertical layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help
            ]
            .as_ref(),
        )
        .split(f.area());

    render_title(f, main_chunks[0]);

    // Content Split (List vs Details)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(30), // List
                Constraint::Percentage(70), // Details
            ]
            .as_ref(),
        )
        .split(main_chunks[1]);

    render_demo_list(f, content_chunks[0], app);
    render_details_pane(f, content_chunks[1], app);
    render_help_bar(f, main_chunks[2]);
}

fn handle_input(
    key: event::KeyEvent,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Down => app.next(),
        KeyCode::Up => app.previous(),
        KeyCode::Enter => {
            if let Some(i) = app.state.selected() {
                let demo = &DEMOS[i];

                // Temporarily restore terminal
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;

                let _ = run_demo(demo.example_name, true);

                // Re-enable TUI
                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.hide_cursor()?;
                terminal.clear()?;
            }
        }
        _ => {}
    }
    Ok(false)
}

fn render_title(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let title = Paragraph::new("✨ ABRASH ENGINE DASHBOARD ✨")
        .style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, area);
}

fn render_demo_list(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &mut App) {
    let items: Vec<ListItem> = DEMOS
        .iter()
        .map(|demo| {
            ListItem::new(Span::styled(
                format!("{} {}", demo.category.icon(), demo.name),
                Style::default().fg(Color::White),
            ))
        })
        .collect();

    let items_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Demos "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(items_list, area, &mut app.state);
}

fn render_details_pane(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &App) {
    if let Some(i) = app.state.selected() {
        let demo = &DEMOS[i];

        let rows = vec![
            Row::new(vec![
                Span::styled(
                    "Description",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(demo.description),
            ])
            .height(2),
            Row::new(vec![
                Span::styled(
                    "Category",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{} {:?}", demo.category.icon(), demo.category)),
            ]),
            Row::new(vec![
                Span::styled(
                    "Instructions",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(demo.instructions),
            ])
            .height(4), // Give instructions some space
            Row::new(vec![
                Span::styled(
                    "Command",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    demo_command(demo.example_name, true),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];

        let table = Table::new(
            rows,
            [Constraint::Length(15), Constraint::Min(0)], // Columns width
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Details "),
        )
        .column_spacing(1);

        f.render_widget(table, area);
    } else {
        let placeholder = Paragraph::new("Select a demo to view details")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(placeholder, area);
    }
}

fn render_help_bar(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let help = Paragraph::new(" ↑/↓: Select | Enter: Launch | Q: Quit ")
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::NONE)); // Flat look for status bar
    f.render_widget(help, area);
}

fn print_launch_header(name: &str) {
    let mut table = ComfyTable::new();
    table
        .load_preset(ComfyPresets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            ComfyCell::new("🚀 Launching Demo")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Cyan),
        ])
        .add_row(vec![ComfyCell::new(format!(
            "Preparing to launch '{name}'..."
        ))]);
    println!("\n{table}");
}

fn print_launch_success() {
    let mut success_table = ComfyTable::new();
    success_table
        .load_preset(ComfyPresets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            ComfyCell::new("✅ Demo Exited Successfully")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Green),
        ]);
    println!("\n{success_table}");
    println!("\n{}", "Press Enter to return to dashboard...".grey());
    let _ = std::io::stdin().read_line(&mut String::new());
}

fn print_launch_error(status: std::process::ExitStatus) {
    let mut error_table = ComfyTable::new();
    error_table
        .load_preset(ComfyPresets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            ComfyCell::new("❌ Demo Crashed")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Red),
        ])
        .add_row(vec![
            ComfyCell::new(format!("Exit Status: {status}")).fg(ComfyColor::Yellow),
        ]);

    println!("\n{error_table}");

    // Give user a chance to read the error
    println!("\n{}", "Press Enter to return to dashboard...".grey());
    let _ = std::io::stdin().read_line(&mut String::new());
}

fn run_demo(name: &str, use_tui_backend: bool) -> Result<(), Box<dyn Error>> {
    print_launch_header(name);

    let args = build_demo_command_args(name, use_tui_backend);
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);

    println!(); // Spacer

    let mut child = cmd.spawn()?;

    let status = child.wait()?;

    if status.success() {
        print_launch_success();
    } else {
        print_launch_error(status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_accepts_tui_flag() {
        let parsed = Args::try_parse_from(["abrash", "--tui"]);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_demo_command_uses_default_features_for_cpu() {
        assert_eq!(
            demo_command("cube_3d", false),
            "cargo run --release --example cube_3d"
        );
    }

    #[test]
    fn test_demo_command_uses_backend_tui_when_requested_for_cpu() {
        assert_eq!(
            demo_command("cube_3d", true),
            "cargo run --release --example cube_3d --no-default-features --features backend-tui"
        );
    }

    #[test]
    fn test_demo_command_keeps_gpu_render_feature_in_tui_mode() {
        assert_eq!(
            demo_command("gpu_cube", true),
            "cargo run --release --example gpu_cube --features gpu-render"
        );
    }

    #[test]
    fn test_demo_command_preserves_nova_feature_in_tui_mode() {
        assert_eq!(
            demo_command("vision_demo", true),
            "cargo run --release --example vision_demo --no-default-features --features backend-tui,nova"
        );
    }
}
