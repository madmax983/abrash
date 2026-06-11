//! Abrash Graphics Demo - Laplacian Texture Blending
//!
//! Demonstrates GPU-Friendly Laplacian Texture Blending
//! (JCGT Vol. 14, No. 1, 2025) vs. naive linear blending.
//!
//! Left quad:  Direct linear blend (`num_levels` = 0). Shows contrast loss and
//!             ghosting where the two textures overlap.
//! Right quad: Laplacian pyramid blend (`num_levels` = 4). Preserves sharp local
//!             features and avoids contrast loss at the blend boundary.
//!
//! Both quads use identical textures and the same smooth blend mask.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_laplacian_blend;
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

const WIDTH: u32 = 900;
const HEIGHT: u32 = 500;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<&'static str> for AppError {
    fn from(s: &'static str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<IoError> for AppError {
    fn from(e: IoError) -> Self {
        Self(e.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(e: abrash::platform::HostError) -> Self {
        Self(e.to_string())
    }
}

fn print_banner() {
    println!("\n{}", "Laplacian Texture Blending Demo".bold().cyan());
    println!("{}", "================================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Paper"),
            Cell::new("JCGT Vol. 14, No. 1, 2025").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Left quad"),
            Cell::new("Direct linear blend (num_levels=0) — ghosting/contrast loss").fg(Color::Red),
        ])
        .add_row(vec![
            Cell::new("Right quad"),
            Cell::new("Laplacian pyramid blend (num_levels=4) — sharp, no ghosting")
                .fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Textures"),
            Cell::new("Low-freq checker + high-freq brick, smooth gradient mask").fg(Color::Yellow),
        ]);

    println!("\n{}", "Info".bold());
    println!("{table}");
    println!("\n{}", "Controls".bold());
    println!(" • Auto-rotating quads for perspective variety\n");
}

/// Generate a low-frequency checker pattern (large squares, contrasting colors).
fn make_checker_texture(size: u32, tile: u32, c0: u32, c1: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(size, size)?;
    for y in 0..size {
        for x in 0..size {
            let color = if ((x / tile) + (y / tile)) & 1 == 0 {
                c0
            } else {
                c1
            };
            tex.set_pixel(x, y, color);
        }
    }
    tex.generate_mipmaps();
    Ok(tex)
}

/// Generate a brick-like pattern using horizontal stripes with offset seams.
fn make_brick_texture(size: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(size, size)?;
    let brick_h = size / 16;
    let brick_w = size / 8;
    let mortar = 0xFF_60_60_60u32; // grey mortar
    let brick = 0xFF_CC_55_33u32; // terracotta
    for y in 0..size {
        let row = y / brick_h;
        let offset = if row & 1 == 0 { 0 } else { brick_w / 2 };
        for x in 0..size {
            let is_mortar_y = (y % brick_h) == 0;
            let is_mortar_x = ((x + offset) % brick_w) == 0;
            tex.set_pixel(
                x,
                y,
                if is_mortar_y || is_mortar_x {
                    mortar
                } else {
                    brick
                },
            );
        }
    }
    tex.generate_mipmaps();
    Ok(tex)
}

/// Generate a smooth horizontal gradient mask: left = 0 (tex0), right = 255 (tex1).
/// A central band of width `transition_frac * size` blends smoothly between them.
fn make_gradient_mask(size: u32, transition_frac: f32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(size, size)?;
    let half = size as f32 * 0.5;
    let half_width = size as f32 * transition_frac * 0.5;
    for y in 0..size {
        for x in 0..size {
            let t = ((x as f32 - half + half_width) / (2.0 * half_width)).clamp(0.0, 1.0);
            // Smooth-step for a natural transition
            let t_smooth = t * t * (3.0 - 2.0 * t);
            let v = (t_smooth * 255.0) as u32;
            tex.set_pixel(x, y, 0xFF_00_00_00 | (v << 16) | (v << 8) | v);
        }
    }
    tex.generate_mipmaps();
    Ok(tex)
}

/// Draw a quad (two triangles) using the given fill function.
#[allow(clippy::too_many_arguments)]
fn draw_quad_laplacian(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    vp: Mat4,
    model: Mat4,
    p: [Vec3; 4],
    uv: [Vec2; 4],
    tex0: &Texture,
    tex1: &Texture,
    mask: &Texture,
    num_levels: usize,
) {
    let mvp = model * vp;
    let v: Vec<_> = p
        .iter()
        .zip(uv.iter())
        .map(|(&pos, &uv_coord)| (mvp.transform_point(pos), uv_coord))
        .collect();

    fill_triangle_laplacian_blend(fb, zb, v[0], v[1], v[2], tex0, tex1, mask, num_levels);
    fill_triangle_laplacian_blend(fb, zb, v[0], v[2], v[3], tex0, tex1, mask, num_levels);
}

struct LaplacianBlendDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    tex0: Texture,
    tex1: Texture,
    mask: Texture,
    projection: Mat4,
    view: Mat4,
    timestep: FixedTimestep,
    angle: f32,
}

impl LaplacianBlendDemoApp {
    fn new() -> Result<Self, AppError> {
        // Low-frequency checker: big teal/orange squares
        let tex0 = make_checker_texture(256, 32, 0xFF_00_AA_AA, 0xFF_FF_88_00)?;
        // High-frequency brick pattern
        let tex1 = make_brick_texture(256)?;
        // Wide smooth gradient mask (60% of width is the transition zone)
        let mask = make_gradient_mask(256, 0.6)?;

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            tex0,
            tex1,
            mask,
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            view: Mat4::look_at(
                Vec3::new(0.0, 0.0, 4.5),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            timestep: FixedTimestep::new(60),
            angle: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for LaplacianBlendDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Laplacian Texture Blending (JCGT 2025)".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        Ok(())
    }

    fn update(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle += 0.008;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_10_10_18);
        self.zbuffer.clear();

        let vp = self.projection * self.view;

        // Quad corners in local space (a flat plane facing the camera)
        let quad_p = [
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let quad_uv = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];

        // Left quad — direct linear blend (num_levels = 0, no Laplacian)
        let model_left = Mat4::rotation_y(self.angle * 0.7) * Mat4::translation(-1.6, 0.0, 0.0);
        draw_quad_laplacian(
            &mut self.framebuffer,
            &mut self.zbuffer,
            vp,
            model_left,
            quad_p,
            quad_uv,
            &self.tex0,
            &self.tex1,
            &self.mask,
            0, // Direct linear blend
        );

        // Right quad — Laplacian pyramid blend (`num_levels` = 4)
        let model_right = Mat4::rotation_y(self.angle * 0.7) * Mat4::translation(1.6, 0.0, 0.0);
        draw_quad_laplacian(
            &mut self.framebuffer,
            &mut self.zbuffer,
            vp,
            model_right,
            quad_p,
            quad_uv,
            &self.tex0,
            &self.tex1,
            &self.mask,
            4, // Laplacian pyramid blend
        );

        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(LaplacianBlendDemoApp::new().unwrap());
    Ok(())
}
