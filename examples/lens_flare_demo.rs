#[cfg(all(feature = "nova", feature = "backend-tui"))]
mod app {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::Vec2;
    use abrash::platform::tui::TuiWindow;
    use abrash_render::experimental::lens_flare::{LensFlareConfig, apply_lens_flare};
    use comfy_table::{Cell, Color, Table, presets};
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::style::Stylize;
    use std::time::Duration;

    const WIDTH: u32 = 160;
    const HEIGHT: u32 = 60;

    fn print_banner() {
        println!("\n{}", "🌟 Lens Flare Demo".bold().cyan());
        println!("{}", "=====================".dark_grey());

        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Property").fg(Color::Cyan),
                Cell::new("Value").fg(Color::Cyan),
            ])
            .add_row(vec![
                Cell::new("Description"),
                Cell::new("Screen-space lens flare artifacts").fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new("Effect"),
                Cell::new("Bright light sources create internal lens reflections")
                    .fg(Color::Yellow),
            ]);

        println!("\n{}", "⚙️  Info".bold());
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
                Cell::new("Arrow Keys"),
                Cell::new("Move the light source"),
            ])
            .add_row(vec![Cell::new("Q / Esc"), Cell::new("Quit Demo")]);
        println!("{controls}\n");
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        print_banner();
        let mut window = TuiWindow::new("Lens Flare Demo", WIDTH, HEIGHT)?;
        let mut fb = Framebuffer::new(WIDTH, HEIGHT)?;

        let mut light_x = WIDTH as f32 / 2.0;
        let mut light_y = HEIGHT as f32 / 2.0;

        let config = LensFlareConfig::default();

        loop {

            #[allow(clippy::collapsible_if)]
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => light_y -= 4.0,
                    KeyCode::Down => light_y += 4.0,
                    KeyCode::Left => light_x -= 4.0,
                    KeyCode::Right => light_x += 4.0,
                    _ => {}
                }
            }
        }

            // Draw a simple dark scene
            fb.clear(0xFF11_1111);

            // Draw the sun/light source
            let sun_r = 15;
            for y in (light_y as i32 - sun_r)..=(light_y as i32 + sun_r) {
                for x in (light_x as i32 - sun_r)..=(light_x as i32 + sun_r) {
                    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                        let dx = x as f32 - light_x;
                        let dy = y as f32 - light_y;
                        if dx * dx + dy * dy <= (sun_r * sun_r) as f32 {
                            fb.set_pixel(x, y, 0xFFFF_FFFF);
                        }
                    }
                }
            }

            // Draw a simple horizon
            let horizon = HEIGHT as i32 - 10;
            for y in horizon..HEIGHT as i32 {
                for x in 0..WIDTH as i32 {
                    fb.set_pixel(x, y, 0xFF22_4422); // Dark green ground
                }
            }

            // Apply Lens Flare
            apply_lens_flare(&mut fb, Vec2::new(light_x, light_y), &config);

            window.blit_framebuffer(&fb);
        }

        Ok(())
    }
}

fn main() {
    #[cfg(all(feature = "nova", feature = "backend-tui"))]
    {
        if let Err(e) = app::run() {
            let mut error_table = comfy_table::Table::new();
            error_table
                .load_preset(comfy_table::presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_header(vec![
                    comfy_table::Cell::new("❌ Application Error")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(comfy_table::Color::Red),
                ])
                .add_row(vec![
                    comfy_table::Cell::new(format!("{e}")).fg(comfy_table::Color::Yellow),
                ]);

            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    }
    #[cfg(not(all(feature = "nova", feature = "backend-tui")))]
    {
        let mut error_table = comfy_table::Table::new();
        error_table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                comfy_table::Cell::new("⚠️  Missing Features: Nova, Backend-TUI")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(comfy_table::Color::Red),
            ])
            .add_row(vec![
                comfy_table::Cell::new("This example requires both 'nova' and 'backend-tui' features to run.")
                    .fg(comfy_table::Color::White),
            ])
            .add_row(vec![
                comfy_table::Cell::new(
                    "Try running with:\ncargo run --example lens_flare_demo --features nova,backend-tui",
                )
                .fg(comfy_table::Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
