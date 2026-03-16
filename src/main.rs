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
}

#[derive(Debug, Clone, Copy)]
enum DemoCategory {
    Cpu3D,
    Gpu3D,
    Simulation,
    Utility,
}

impl DemoCategory {
    const fn icon(&self) -> char {
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
        name: "Depth Fog",
        category: DemoCategory::Simulation,
        description: "Distance-based depth fog effect",
        instructions: "Mouse: None\nKeyboard: Auto-moving camera",
        example_name: "fog_demo",
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
}

fn demo_command(example_name: &str) -> String {
    if is_gpu_render_example(example_name) {
        format!("cargo run --release --example {example_name} --features gpu-render")
    } else if is_nova_example(example_name) {
        if std::env::consts::OS == "windows" {
            format!("cargo run --release --example {example_name} --features nova")
        } else {
            format!(
                "cargo run --release --example {example_name} --no-default-features --features backend-tui,nova"
            )
        }
    } else if std::env::consts::OS == "windows" {
        format!("cargo run --release --example {example_name}")
    } else {
        format!(
            "cargo run --release --example {example_name} --no-default-features --features backend-tui"
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if let Some(demo_name) = args.demo {
        run_demo(&demo_name)?;
        return Ok(());
    }

    if args.list {
        print_demo_list();
        return Ok(());
    }

    // TUI Mode
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
    table.load_preset(ComfyPresets::UTF8_FULL).set_header(vec![
        ComfyCell::new("Icon").add_attribute(comfy_table::Attribute::Bold),
        ComfyCell::new("Name").add_attribute(comfy_table::Attribute::Bold),
        ComfyCell::new("Category").add_attribute(comfy_table::Attribute::Bold),
        ComfyCell::new("Description").add_attribute(comfy_table::Attribute::Bold),
        ComfyCell::new("Command").add_attribute(comfy_table::Attribute::Bold),
    ]);

    for demo in DEMOS {
        table.add_row(vec![
            ComfyCell::new(demo.category.icon()),
            ComfyCell::new(demo.name).fg(ComfyColor::Cyan),
            ComfyCell::new(format!("{:?}", demo.category)).fg(ComfyColor::Yellow),
            ComfyCell::new(demo.description),
            ComfyCell::new(format!("abrash --demo {}", demo.example_name)).fg(ComfyColor::DarkGrey),
        ]);
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
        terminal.draw(|f| {
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

            // Title
            let title = Paragraph::new("✨ ABRASH ENGINE DASHBOARD ✨")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(title, main_chunks[0]);

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

            // Demo List
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
                .block(Block::default().borders(Borders::ALL).title(" Demos "))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(items_list, content_chunks[0], &mut app.state);

            // Details Pane
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
                            demo_command(demo.example_name),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                ];

                let table = Table::new(
                    rows,
                    [Constraint::Length(15), Constraint::Min(0)], // Columns width
                )
                .block(Block::default().borders(Borders::ALL).title(" Details "))
                .column_spacing(1);

                f.render_widget(table, content_chunks[1]);
            } else {
                let placeholder = Paragraph::new("Select a demo to view details")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(placeholder, content_chunks[1]);
            }

            // Help Bar
            let help = Paragraph::new(" ↑/↓: Select | Enter: Launch | Q: Quit ")
                .style(Style::default().fg(Color::White).bg(Color::DarkGray))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::NONE)); // Flat look for status bar
            f.render_widget(help, main_chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
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

                        let _ = run_demo(demo.example_name);

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
        }
    }
}

fn run_demo(name: &str) -> Result<(), Box<dyn Error>> {
    let mut table = ComfyTable::new();
    table
        .load_preset(ComfyPresets::UTF8_FULL)
        .set_header(vec![
            ComfyCell::new("🚀 Launching Demo")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Cyan),
        ])
        .add_row(vec![ComfyCell::new(format!(
            "Preparing to launch '{name}'..."
        ))]);
    println!("\n{table}");

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--release").arg("--example").arg(name);

    if is_gpu_render_example(name) {
        cmd.arg("--features").arg("gpu-render");
    } else if is_nova_example(name) {
        cmd.arg("--features").arg("nova");
    }

    // Smart Launch: On non-Windows systems, default to TUI backend to ensure
    // the example runs (as Win32 API is not available).
    if std::env::consts::OS != "windows" && !is_gpu_render_example(name) {
        let mut info_table = ComfyTable::new();
        info_table
            .load_preset(ComfyPresets::UTF8_FULL)
            .add_row(vec![
                ComfyCell::new("ℹ️  Non-Windows OS detected").fg(ComfyColor::Blue),
                ComfyCell::new("Enabling TUI backend...").fg(ComfyColor::DarkGrey),
            ]);
        println!("{info_table}");

        cmd.arg("--no-default-features")
            .arg("--features")
            .arg("backend-tui");
    }

    println!(); // Spacer

    let mut child = cmd.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        let mut error_table = ComfyTable::new();
        error_table
            .load_preset(ComfyPresets::UTF8_FULL)
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

    Ok(())
}
