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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{error::Error, io, process::Command};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the demo to run directly
    #[arg(long, short)]
    demo: Option<String>,

    /// List available demos
    #[arg(long, short)]
    list: bool,
}

struct Demo {
    name: &'static str,
    description: &'static str,
    example_name: &'static str,
}

const DEMOS: &[Demo] = &[
    Demo {
        name: "Lit Cube",
        description: "Flat shaded cube with directional lighting",
        example_name: "lit_cube",
    },
    Demo {
        name: "Cube 3D",
        description: "Basic 3D cube rendering",
        example_name: "cube_3d",
    },
    Demo {
        name: "Obj Viewer",
        description: "Load and view 3D objects (embedded spaceship)",
        example_name: "obj_viewer",
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.list {
        println!("Available Demos:");
        for demo in DEMOS {
            println!("  {: <12} - {}", demo.name, demo.description);
        }
        return Ok(());
    }

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
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let title = Paragraph::new("Abrash Engine Dashboard")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Double));
            f.render_widget(title, chunks[0]);

            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(chunks[1]);

            let items: Vec<ListItem> = DEMOS
                .iter()
                .map(|demo| {
                    ListItem::new(Line::from(vec![Span::styled(
                        format!(" {} ", demo.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    )]))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Demos "))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, content_chunks[0], &mut app.state);

            let selected_index = app.state.selected().unwrap_or(0);
            let selected_demo = &DEMOS[selected_index];

            let details_text = vec![
                Line::from(Span::styled("Description:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))),
                Line::from(Span::raw("")),
                Line::from(Span::raw(selected_demo.description)),
                Line::from(Span::raw("")),
                Line::from(Span::styled("Command:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))),
                Line::from(Span::raw(format!("cargo run --example {}", selected_demo.example_name))),
            ];

            let details = Paragraph::new(details_text)
                .block(Block::default().borders(Borders::ALL).title(" Details "))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(details, content_chunks[1]);

            let help = Paragraph::new("↑/↓: Navigate | Enter: Launch | Q: Quit")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::TOP));
            f.render_widget(help, chunks[2]);
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
    println!("Launching {name}...");
    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--example")
        .arg(name)
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        eprintln!("Demo exited with error: {status}");
        eprintln!("Press Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }

    Ok(())
}
