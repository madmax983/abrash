//! Arboretum CLI - A TUI dashboard for the Arboretum L-System Generator.
//!
//! This example demonstrates how to use the experimental `Arboretum` module
//! to generate procedural plants and visualize the process.

#[cfg(feature = "nova")]
mod app {
    use abrash::experimental::arboretum::LSystem;
    use clap::Parser;
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{
        Terminal,
        backend::CrosstermBackend,
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph, Wrap},
    };
    use std::{collections::HashMap, io, time::Duration};

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        /// Axiom (Initial state)
        #[arg(short, long, default_value = "X")]
        axiom: String,

        /// Rules (format: "A=AB,B=A")
        #[arg(short, long, default_value = "X=F+[[X]-X]-F[-FX]+X,F=FF")]
        rules: String,

        /// Angle in degrees
        #[arg(short = 'g', long, default_value_t = 25.0)]
        angle: f32,

        /// Iterations
        #[arg(short, long, default_value_t = 4)]
        iterations: u32,
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args = Args::parse();

        // Parse rules
        let mut parsed_rules = HashMap::new();
        for rule in args.rules.split(',') {
            if let Some((input, output)) = rule.split_once('=') {
                if let Some(c) = input.chars().next() {
                    parsed_rules.insert(c, output.to_string());
                }
            }
        }

        // Initialize L-System
        let mut lsys = LSystem::new(&args.axiom, args.angle, 0.1, 0.02);
        for (k, v) in &parsed_rules {
            lsys.add_rule(*k, v);
        }

        // Generate
        let start_time = std::time::Instant::now();
        let expanded = lsys.expand(args.iterations)?;
        let mesh = lsys.generate_mesh(args.iterations)?;
        let duration = start_time.elapsed();

        // TUI Setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = run_tui(
            &mut terminal,
            &args,
            &expanded,
            mesh.vertices.len(),
            mesh.indices.len(),
            duration,
            &parsed_rules,
        );

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("TUI Error: {err:?}");
        }

        Ok(())
    }

    fn run_tui(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        args: &Args,
        expanded: &str,
        vertex_count: usize,
        triangle_count: usize,
        duration: Duration,
        rules: &HashMap<char, String>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),      // Title
                            Constraint::Min(10),        // Main Content
                            Constraint::Percentage(30), // DNA (Bottom)
                            Constraint::Length(1),      // Help
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                // Title
                let title = Paragraph::new("🌿 Arboretum: Procedural Generator")
                    .style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center);
                f.render_widget(title, chunks[0]);

                // Split Main Content
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(chunks[1]);

                // Genome Panel
                let mut genome_text = vec![
                    Line::from(vec![
                        Span::styled("Axiom: ", Style::default().fg(Color::Cyan)),
                        Span::raw(&args.axiom),
                    ]),
                    Line::from(vec![
                        Span::styled("Angle: ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{:.1}°", args.angle)),
                    ]),
                    Line::from(Span::styled("Rules:", Style::default().fg(Color::Cyan))),
                ];

                for (k, v) in rules {
                    genome_text.push(Line::from(format!("  {k} → {v}")));
                }

                let genome_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" 🧬 Genome (Config) ")
                    .border_style(Style::default().fg(Color::Blue));
                let genome = Paragraph::new(genome_text).block(genome_block);
                f.render_widget(genome, main_chunks[0]);

                // Analysis Panel
                let analysis_text = vec![
                    Line::from(vec![
                        Span::styled("Iterations: ", Style::default().fg(Color::Yellow)),
                        Span::raw(args.iterations.to_string()),
                    ]),
                    Line::from(vec![
                        Span::styled("Generation Time: ", Style::default().fg(Color::Yellow)),
                        Span::raw(format!("{:.2?}", duration)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Mesh Statistics:",
                        Style::default().add_modifier(Modifier::UNDERLINED),
                    )),
                    Line::from(vec![
                        Span::styled("  Vertices: ", Style::default().fg(Color::Magenta)),
                        Span::raw(vertex_count.to_string()),
                    ]),
                    Line::from(vec![
                        Span::styled("  Triangles: ", Style::default().fg(Color::Magenta)),
                        Span::raw(triangle_count.to_string()),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  DNA Length: ", Style::default().fg(Color::Magenta)),
                        Span::raw(format!("{} chars", expanded.len())),
                    ]),
                ];

                let analysis_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" 📊 Analysis ")
                    .border_style(Style::default().fg(Color::Yellow));
                let analysis = Paragraph::new(analysis_text).block(analysis_block);
                f.render_widget(analysis, main_chunks[1]);

                // DNA Sequence (Bottom)
                // Colorize the DNA string
                // F, f -> Green (Growth)
                // +, -, &, ^, \, / -> Yellow (Rotation)
                // [, ] -> Blue (Structure)
                // Others -> White
                let mut styled_dna = Vec::new();
                let mut current_span = String::new();
                let mut current_color = Color::White;

                // Optimization: Don't render huge strings entirely, just a preview
                let preview_len = expanded.len().min(2000);
                let display_str = &expanded[..preview_len];

                for c in display_str.chars() {
                    let color = match c {
                        'F' | 'f' => Color::Green,
                        '+' | '-' | '&' | '^' | '\\' | '/' | '|' => Color::Yellow,
                        '[' | ']' => Color::Blue,
                        _ => Color::White,
                    };

                    if color != current_color && !current_span.is_empty() {
                        styled_dna.push(Span::styled(
                            current_span.clone(),
                            Style::default().fg(current_color),
                        ));
                        current_span.clear();
                    }
                    current_color = color;
                    current_span.push(c);
                }
                if !current_span.is_empty() {
                    styled_dna.push(Span::styled(
                        current_span,
                        Style::default().fg(current_color),
                    ));
                }

                if expanded.len() > preview_len {
                    styled_dna.push(Span::styled("...", Style::default().fg(Color::DarkGray)));
                }

                let dna_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" 🧬 DNA Sequence ")
                    .border_style(Style::default().fg(Color::Magenta));

                let dna_paragraph = Paragraph::new(Line::from(styled_dna))
                    .block(dna_block)
                    .wrap(Wrap { trim: true });
                f.render_widget(dna_paragraph, chunks[2]);

                // Help Bar
                let help = Paragraph::new(" Press [Q] to Quit ")
                    .style(Style::default().fg(Color::Black).bg(Color::White))
                    .alignment(Alignment::Center);
                f.render_widget(help, chunks[3]);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() {
    #[cfg(feature = "nova")]
    {
        if let Err(e) = app::run() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "nova"))]
    {
        eprintln!("This example requires the 'nova' feature.");
        eprintln!("Run with: cargo run --example arboretum_cli --features nova");
    }
}
