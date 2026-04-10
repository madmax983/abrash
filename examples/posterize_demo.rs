#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unnecessary_wraps)]
#[cfg(feature = "nova")]
use abrash::experimental::posterize::{PosterizeConfig, apply_posterize};
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::texture::Texture;
#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🎨 Posterize Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Posterize image effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Quantizes colors to specific levels").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("None"),
            Cell::new("Static terminal output demo"),
        ]);
    println!("{controls}\n");
}

fn main() {
    #[cfg(feature = "nova")]
    {
        print_banner();
        let width = 256;
        let height = 256;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Create a simple gradient texture directly if we don't have an image to load easily
        // Or we can try to use the texture module.
        // Let's just create a texture programmatically for the demo
        let mut tex = Texture::new(width, height).unwrap();
        for y in 0..height {
            for x in 0..width {
                let r = (x % 256) << 16;
                let g = (y % 256) << 8;
                let b = (x + y) % 256;
                tex.set_pixel(x, y, 0xFF00_0000 | r | g | b);
            }
        }

        // Copy texture to framebuffer
        for y in 0..height {
            for x in 0..width {
                let p = tex.get_pixel(x as f32 / width as f32, y as f32 / height as f32);
                fb.set_pixel(x as i32, y as i32, p);
            }
        }

        println!("Applying posterize effect with 4 levels...");
        let config = PosterizeConfig { levels: 4.0 };
        apply_posterize(&mut fb, &config);

        println!("Applying posterize effect with 8 levels...");
        let config2 = PosterizeConfig { levels: 8.0 };
        apply_posterize(&mut fb, &config2);

        println!("Posterize effect successfully applied to demo framebuffer!");
    }
}
