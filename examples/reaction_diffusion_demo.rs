//! Reaction-Diffusion (Gray-Scott) Demo
//!
//! Simulates the Turing patterns formed by two interacting chemicals.
//! Uses a double-buffered grid and 3x3 Laplacian convolutions.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::utils::XorShift32;

use abrash_render::experimental::reaction_diffusion::ReactionDiffusion;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

struct ReactionDiffusionDemo {
    sim: ReactionDiffusion,
    color_a: u32,
    color_b: u32,
    presenter: Option<SoftwarePresenter>,
}

impl ReactionDiffusionDemo {
    fn new() -> Self {
        let width = 800;
        let height = 600;

        let sim_w = (width / 2) as usize; // run at half-res for speed and thicker lines
        let sim_h = (height / 2) as usize;

        let mut sim = ReactionDiffusion::new(sim_w, sim_h);

        // Seed some initial spots
        let mut rng = XorShift32::new(42);

        sim.seed(sim_w / 2, sim_h / 2, 10);
        sim.seed(sim_w / 3, sim_h / 3, 5);
        sim.seed(sim_w * 2 / 3, sim_h * 2 / 3, 8);
        sim.seed_random(&mut rng, sim_w / 4);

        Self {
            sim,
            color_a: 0xFF_110022, // Deep purple/black
            color_b: 0xFF_00FFAA, // Bright cyan/green
            presenter: None,
        }
    }
}

impl WindowApp for ReactionDiffusionDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Reaction-Diffusion (Gray-Scott)".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    fn update(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        if self.presenter.is_none() {
            self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        }

        // We can run multiple steps per frame to speed up the visual evolution
        for _ in 0..10 {
            self.sim.step();
        }

        let presenter = self.presenter.as_mut().unwrap();

        let fb_width = 800;
        let fb_height = 600;

        // We run the sim at half-res, but we'll manually render it to fill the fb.
        let w = self.sim.width.min(fb_width as usize / 2);
        let h = self.sim.height.min(fb_height as usize / 2);

        // To avoid having to implement a scaling render, let's just make a temp fb,
        // render the sim to it, and nearest-neighbor scale it up.
        let mut temp_fb = Framebuffer::new(w as u32, h as u32).unwrap();
        self.sim.render(&mut temp_fb, self.color_a, self.color_b);

        let mut fb_owned = Framebuffer::new(fb_width, fb_height).unwrap();

        // Clear background
        fb_owned.clear(self.color_a);

        let fb_w = fb_owned.width() as usize;
        let fb_h = fb_owned.height() as usize;
        let fb_pixels = fb_owned.as_mut_slice();

        for y in 0..h {
            for x in 0..w {
                let pixel = temp_fb.get_pixel(x as i32, y as i32).unwrap_or(0);

                // 2x Nearest Neighbor Scale
                let dest_y = y * 2;
                let dest_x = x * 2;

                if dest_y + 1 < fb_h && dest_x + 1 < fb_w {
                    fb_pixels[dest_y * fb_w + dest_x] = pixel;
                    fb_pixels[dest_y * fb_w + dest_x + 1] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x] = pixel;
                    fb_pixels[(dest_y + 1) * fb_w + dest_x + 1] = pixel;
                }
            }
        }

        presenter.present(&fb_owned)?;
        ctx.window.request_redraw();

        Ok(())
    }
}

fn print_banner() {
    println!(
        "\n{}",
        "🌟 Reaction-Diffusion (Gray-Scott) Demo".bold().cyan()
    );
    println!("{}", "=======================================".dark_grey());

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
            Cell::new("Simulates Turing patterns formed by two interacting chemicals")
                .fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Post-Process").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None")]);
    println!("{controls}\n");
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    run_windowed(ReactionDiffusionDemo::new());
    Ok(())
}
