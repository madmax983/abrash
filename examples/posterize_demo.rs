use abrash::experimental::posterize::{PosterizeConfig, apply_posterize};
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

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
    print_banner();

    let width = 256;
    let height = 256;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Create a simple gradient texture directly if we don't have an image to load easily
    // Or we can try to use the texture module.
    // Let's just create a texture programmatically for the demo
    let mut tex = Texture::new(width as u32, height as u32).unwrap();
    for y in 0..height as u32 {
        for x in 0..width as u32 {
            let r = (x % 256) << 16;
            let g = (y % 256) << 8;
            let b = (x + y) % 256;
            tex.set_pixel(x, y, 0xFF000000 | r | g | b);
        }
    }

    // Copy texture to framebuffer
    for y in 0..height {
        for x in 0..width {
            let p = tex.get_pixel(x as f32 / width as f32, y as f32 / height as f32);
            fb.set_pixel(x as i32, y as i32, p);
        }
    }

    println!("{} Applying posterize effect with 4 levels...", "▶️".cyan());
    let config = PosterizeConfig { levels: 4.0 };
    apply_posterize(&mut fb, &config);

    println!("{} Applying posterize effect with 8 levels...", "▶️".cyan());
    let config2 = PosterizeConfig { levels: 8.0 };
    apply_posterize(&mut fb, &config2);

    println!(
        "\n{}",
        "✅ Posterize effect successfully applied to demo framebuffer!"
            .bold()
            .green()
    );
}
