//! Abrash Engine - CLI Dashboard & Launcher
//!
//! Provides a TUI interface to explore and launch demos.

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{error::Error, io, process::Command};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the demo to run directly
    #[arg(long, short)]
    demo: Option<String>,
}

struct Demo {
    name: &'static str,
    description: &'static str,
    instructions: &'static str,
    example_name: &'static str,
}

const DEMOS: &[Demo] = &[
    Demo {
        name: "Lit Cube",
        description: "Flat shaded cube with directional lighting",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "lit_cube",
    },
    Demo {
        name: "Cube 3D",
        description: "Basic 3D cube rendering",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "cube_3d",
    },
    Demo {
        name: "OBJ Viewer",
        description: "Loads and renders a 3D model (Spaceship)",
        instructions: "• Mouse: None\n• Keyboard: Auto-rotating",
        example_name: "obj_viewer",
    },
    Demo {
        name: "GPU Cube",
        description: "Hardware-accelerated cube rendering with wgpu",
        instructions: "Mouse: Drag to orbit, wheel to zoom\nKeyboard: Arrows/WASD orbit, Q/E zoom, Space toggle auto-rotate, R reset",
        example_name: "gpu_cube",
    },
];

fn demo_command(example_name: &str) -> String {
    if example_name == "gpu_cube" {
        "cargo run --release --example gpu_cube --features gpu-render".to_string()
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

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

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
            let title = Paragraph::new("Abrash Engine Dashboard")
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
                        Constraint::Percentage(40), // List
                        Constraint::Percentage(60), // Details
                    ]
                    .as_ref(),
                )
                .split(main_chunks[1]);

            // Demo List
            let items: Vec<ListItem> = DEMOS
                .iter()
                .map(|demo| {
                    ListItem::new(Span::styled(
                        format!(" {} ", demo.name),
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

                let details_text = vec![
                    Line::from(Span::styled(
                        "Description:",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    )),
                    Line::from(format!("  {}\n", demo.description)),
                    Line::from(Span::styled(
                        "Instructions:",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    )),
                    Line::from(format!("{}\n", demo.instructions)),
                    Line::from(Span::styled(
                        "Command:",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    )),
                    Line::from(Span::styled(
                        format!("  {}", demo_command(demo.example_name)),
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                let details = Paragraph::new(details_text)
                    .block(Block::default().borders(Borders::ALL).title(" Details "))
                    .wrap(Wrap { trim: true });
                f.render_widget(details, content_chunks[1]);
            } else {
                let placeholder = Paragraph::new("Select a demo to view details")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::DarkGray));
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
    println!("Preparing to launch {name}...");

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--release").arg("--example").arg(name);

    if name == "gpu_cube" {
        cmd.arg("--features").arg("gpu-render");
    }

    // Smart Launch: On non-Windows systems, default to TUI backend to ensure
    // the example runs (as Win32 API is not available).
    if std::env::consts::OS != "windows" && name != "gpu_cube" {
        println!(
            "ℹ️  Non-Windows OS detected ({}). Enabling TUI backend...",
            std::env::consts::OS
        );
        cmd.arg("--no-default-features")
            .arg("--features")
            .arg("backend-tui");
    }

    let mut child = cmd.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        eprintln!("❌ Demo exited with error: {status}");
        // Give user a chance to read the error
        println!("Press Enter to return to dashboard...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    Ok(())
}
