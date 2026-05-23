//! GPU MVP cube demo using the shared native host seam.

use abrash::platform::{WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_gpu_render::surface::GpuSurface;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
use abrash_render::render_api::material::Material;
use std::error::Error;
use std::fmt;
use std::time::Instant;

#[derive(Debug)]
struct DemoError(String);

impl fmt::Display for DemoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for DemoError {}

impl From<String> for DemoError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

struct GpuMvpCubeApp {
    renderer: Option<GpuRenderer>,
    surface: Option<GpuSurface>,
    mesh: Option<MeshHandle>,
    material: Option<MaterialHandle>,
    start: Instant,
}

impl GpuMvpCubeApp {
    fn new() -> Self {
        Self {
            renderer: None,
            surface: None,
            mesh: None,
            material: None,
            start: Instant::now(),
        }
    }
}

impl WindowApp for GpuMvpCubeApp {
    type Error = DemoError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash GPU MVP Cube".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        use comfy_table::{Cell, Color, Table, presets};
        use crossterm::style::Stylize;

        println!("\n{}", "🧊 GPU MVP Cube Demo".bold().cyan());
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
                Cell::new("Hardware-accelerated spinning cube").fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new("Renderer"),
                Cell::new("WGPU Backend").fg(Color::Yellow),
            ]);

        println!("\n{}", "⚙️  Info".bold());
        println!("{table}");

        let mut controls = Table::new();
        controls
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Input").fg(Color::Cyan),
                Cell::new("Action").fg(Color::Cyan),
            ])
            .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
            .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

        println!("\n{}", "🎮 Controls".bold());
        println!("{controls}\n");

        let (mut renderer, surface) = GpuRenderer::new_windowed(ctx.window)?;
        let mesh = renderer.create_mesh(Mesh::cube(1.0))?;
        let material = renderer.create_material(Material::flat(0xFFFF_4444));

        self.renderer = Some(renderer);
        self.surface = Some(surface);
        self.mesh = Some(mesh);
        self.material = Some(material);

        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        if let (Some(renderer), Some(surface)) = (self.renderer.as_ref(), self.surface.as_mut()) {
            surface.resize(renderer.device(), width, height);
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| DemoError("renderer not initialized".to_string()))?;
        let surface = self
            .surface
            .as_mut()
            .ok_or_else(|| DemoError("surface not initialized".to_string()))?;
        let mesh = self
            .mesh
            .ok_or_else(|| DemoError("mesh not initialized".to_string()))?;
        let material = self
            .material
            .ok_or_else(|| DemoError("material not initialized".to_string()))?;

        let elapsed = self.start.elapsed().as_secs_f32();
        let angle = elapsed * 0.8;
        let size = ctx.window.inner_size();
        let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;
        let camera = FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 2.0, 5.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.0, aspect, 0.1, 100.0),
        );

        // ⚡ Bolt: Use `with_capacity` to prevent vector reallocations for draw commands
        let mut frame = Frame::with_capacity(camera, 1, 0);
        frame.draw(mesh, material, Mat4::rotation_y(angle));

        renderer
            .render_to_surface(&frame, surface)
            .map_err(DemoError::from)
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), DemoError> {
    run_windowed(GpuMvpCubeApp::new());
    Ok(())
}
