//! MSDF text atlas demo: build one MTSDF atlas from the built-in
//! vector glyphs, then draw the same word at three sizes with it —
//! the edges stay anti-aliased at every size because the shader's
//! range scales with the draw size, not the atlas.
//!
//! Run with: `cargo run -p abrash-core --example msdf_text_demo`
//! Writes `msdf_text_demo.ppm` to the current directory.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_core::msdf::{GeneratorConfig, TextAtlas, demo_glyphs, draw_text, measure_text};

fn main() {
    let atlas = TextAtlas::build(&demo_glyphs(), GeneratorConfig::default(), 256);
    println!(
        "msdf_text_demo: atlas {}x{}, {} glyphs",
        atlas.width,
        atlas.height,
        atlas.glyphs.len()
    );

    let (w, h) = (480, 300);
    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    fb.clear(0xFF10_1018);

    // One atlas, three sizes: caption, headline, display.
    let small = draw_text(
        &mut fb,
        &atlas,
        "SDF",
        Vec2::new(24.0, 52.0),
        28.0,
        Color::grey(0.75),
    );
    let mid = draw_text(
        &mut fb,
        &atlas,
        "MSDF",
        Vec2::new(24.0, 150.0),
        72.0,
        Color::WHITE,
    );
    let big = draw_text(
        &mut fb,
        &atlas,
        "ATLAS",
        Vec2::new(24.0, 272.0),
        104.0,
        Color::rgb(0.45, 0.75, 1.0),
    );
    println!("drawn widths: SDF={small:.1} MSDF={mid:.1} ATLAS={big:.1}");
    println!(
        "measured ATLAS at 104px: {:.1}",
        measure_text(&atlas, "ATLAS", 104.0)
    );

    let partial = fb
        .as_slice()
        .iter()
        .filter(|&&px| px != 0xFF10_1018)
        .count();
    fb.export_ppm("msdf_text_demo.ppm").expect("write PPM");
    println!("msdf_text_demo: {w}x{h}, {partial} non-background pixels, wrote msdf_text_demo.ppm");
}
