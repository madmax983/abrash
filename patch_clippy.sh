sed -i 's/let dist = (dx \* dx + dy \* dy).sqrt();/let dist = dx.hypot(dy);/g' examples/string_art_demo.rs

cat << 'INNER_EOF' > examples/string_art_demo.rs
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::string_art::apply_string_art;

#[cfg(feature = "backend-tui")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "backend-tui")]
use crossterm::style::Stylize;

#[cfg(feature = "backend-tui")]
fn print_banner() {
    println!("\n{}", "🌟 String Art Demo".bold().cyan());
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
            Cell::new("Approximates an image using continuous thread wrapped around pins").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

#[cfg(not(feature = "backend-tui"))]
fn print_banner() {}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: String Art Demo";

struct StringArtDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
    num_lines: usize,
}

impl StringArtDemoApp {
    fn new() -> Self {
        let mut background_fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
        // A simple radial gradient to approximate
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let dx = x as f32 - WIDTH as f32 / 2.0;
                let dy = y as f32 - HEIGHT as f32 / 2.0;
                let dist = dx.hypot(dy);
                let max_dist = WIDTH as f32 / 2.0;

                let mut v = 1.0 - (dist / max_dist).min(1.0); // 1.0 at center, 0.0 at edge
                v = v * v; // make it more pronounced

                let vu8 = (v * 255.0) as u32;
                background_fb.set_pixel(x as i32, y as i32, 0xFF_00_00_00 | (vu8 << 16) | (vu8 << 8) | vu8);
            }
        }

        Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            background_fb,
            time: 0.0,
            num_lines: 50,
        }
    }

    fn present(&mut self) -> Result<(), HostError> {
        let presenter = self.presenter.as_mut().unwrap();
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

impl WindowApp for StringArtDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        // Slowly increase lines to show progress
        self.num_lines = (50 + (self.time * 200.0) as usize).min(2000);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.as_mut_slice().copy_from_slice(self.background_fb.as_slice());
        apply_string_art(&mut self.framebuffer, 256, self.num_lines, 0.05);
        self.present()
    }
}

fn main() {
    print_banner();
    run_windowed(StringArtDemoApp::new());
}
INNER_EOF
