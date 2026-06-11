//! GPU Deferred Rendering Showcase
//!
//! Demonstrates the full deferred rendering pipeline:
//! - Multiple objects with varying PBR materials (metal, plastic, rough, smooth)
//! - Directional light with shadow mapping
//! - Multiple colored point lights
//! - HDR rendering with Reinhard tone mapping
//! - Auto-rotating camera orbit
//!
//! This scene renders through the complete 5-pass pipeline:
//!   Shadow → G-Buffer MRT → Deferred Lighting (IBL) → Skybox → Tone Map

use abrash::platform::{WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::composition::DebugMode;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_gpu_render::surface::GpuSurface;
use abrash_render::render_api::frame::{DirectionalLight, Frame, FrameCamera, Light, PointLight};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
use abrash_render::render_api::material::{Material, ShadingMode};
use std::error::Error;
use std::fmt;
use std::time::Instant;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

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

/// Material presets for the showcase scene.
struct MaterialSet {
    /// Polished chrome (high metallic, low roughness)
    chrome: MaterialHandle,
    /// Brushed gold (metallic with moderate roughness)
    gold: MaterialHandle,
    /// Red plastic (non-metallic, moderate roughness)
    red_plastic: MaterialHandle,
    /// Blue rubber (non-metallic, high roughness)
    blue_rubber: MaterialHandle,
    /// White marble (low roughness, subtle specular)
    marble: MaterialHandle,
    /// Ground plane (dark, rough)
    ground: MaterialHandle,
}

const DEBUG_MODES: [DebugMode; 8] = [
    DebugMode::None,
    DebugMode::GBufferPosition,
    DebugMode::GBufferNormal,
    DebugMode::GBufferAlbedo,
    DebugMode::Roughness,
    DebugMode::Metallic,
    DebugMode::Shadows,
    DebugMode::Depth,
];

const DEBUG_MODE_NAMES: [&str; 8] = [
    "Normal Rendering",
    "G-Buffer: Position",
    "G-Buffer: Normal",
    "G-Buffer: Albedo",
    "G-Buffer: Roughness",
    "G-Buffer: Metallic",
    "Shadows",
    "Depth",
];

struct ShowcaseApp {
    renderer: Option<GpuRenderer>,
    surface: Option<GpuSurface>,
    sphere_mesh: Option<MeshHandle>,
    cube_mesh: Option<MeshHandle>,
    cylinder_mesh: Option<MeshHandle>,
    torus_mesh: Option<MeshHandle>,
    ground_mesh: Option<MeshHandle>,
    materials: Option<MaterialSet>,
    anim_time: f32,
    last_frame: Instant,
    taa_enabled: bool,
    debug_mode_index: usize,
    paused: bool,
}

impl ShowcaseApp {
    fn new() -> Self {
        Self {
            renderer: None,
            surface: None,
            sphere_mesh: None,
            cube_mesh: None,
            cylinder_mesh: None,
            torus_mesh: None,
            ground_mesh: None,
            materials: None,
            anim_time: 0.0,
            last_frame: Instant::now(),
            taa_enabled: false,
            debug_mode_index: 0,
            paused: false,
        }
    }

    /// Build the showcase scene for a given time value.
    fn build_frame(&self, elapsed: f32, aspect: f32) -> Frame {
        // Orbiting camera
        let orbit_speed = 0.3;
        let cam_angle = elapsed * orbit_speed;
        let cam_height = 4.0 + (elapsed * 0.2).sin() * 1.5;
        let cam_dist = 10.0;
        let eye = Vec3::new(
            cam_angle.cos() * cam_dist,
            cam_height,
            cam_angle.sin() * cam_dist,
        );

        let camera = FrameCamera::new(
            Mat4::look_at(eye, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            Mat4::perspective(0.8, aspect, 0.1, 200.0),
        );

        // ⚡ Bolt: Use `with_capacity` to prevent vector reallocations for draw commands and lights
        // 1 directional light + 3 point lights = 4 lights
        // 1 ground + 1 center sphere + 1 orbit + 6 outer objects + 16 inner boxes = 25 meshes
        let mut frame = Frame::with_capacity(camera, 25, 4);
        frame.clear_color = Some(0xFF0A_0A12); // dark blue-black

        // --- Lights ---

        // Warm directional sun (casts shadows)
        frame.add_light(Light::Directional(DirectionalLight {
            direction: Vec3::new(0.4, -0.8, -0.3),
            color: 0xFFF5_E6C8, // warm white
            intensity: 1.2,
        }));

        // Red point light (orbits)
        let red_angle = elapsed * 1.5;
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(red_angle.cos() * 4.0, 2.5, red_angle.sin() * 4.0),
            color: 0xFFFF_3333,
            intensity: 3.0,
            radius: 8.0,
        }));

        // Blue point light (orbits opposite)
        let blue_angle = elapsed * 1.5 + std::f32::consts::PI;
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(blue_angle.cos() * 3.5, 1.5, blue_angle.sin() * 3.5),
            color: 0xFF33_66FF,
            intensity: 2.5,
            radius: 7.0,
        }));

        // Green point light (static, above)
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(0.0, 5.0, 0.0),
            color: 0xFF33_FF66,
            intensity: 1.5,
            radius: 12.0,
        }));

        let mats = self.materials.as_ref().unwrap();
        let sphere = self.sphere_mesh.unwrap();
        let cube = self.cube_mesh.unwrap();
        let cylinder = self.cylinder_mesh.unwrap();
        let torus = self.torus_mesh.unwrap();
        let ground = self.ground_mesh.unwrap();

        // --- Ground plane (proper plane mesh) ---
        frame.draw(ground, mats.ground, Mat4::translation(0.0, -0.5, 0.0));

        // --- Center pedestal: large chrome sphere ---
        let bob = (elapsed * 0.8).sin() * 0.3;
        frame.draw(
            sphere,
            mats.chrome,
            Mat4::translation(0.0, 1.5 + bob, 0.0) * Mat4::scale(1.2, 1.2, 1.2),
        );

        // --- Ring of objects ---
        let ring_count = 8;
        for i in 0..ring_count {
            let angle = (i as f32 / ring_count as f32) * std::f32::consts::TAU;
            let radius = 4.5;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let spin = elapsed * (0.5 + i as f32 * 0.1);

            let (mesh, mat) = match i % 8 {
                0 => (cube, mats.gold),
                1 => (sphere, mats.red_plastic),
                2 => (cylinder, mats.blue_rubber),
                3 => (torus, mats.marble),
                4 => (sphere, mats.chrome),
                5 => (cube, mats.red_plastic),
                6 => (cylinder, mats.gold),
                _ => (torus, mats.chrome),
            };

            let scale = 0.6 + (i as f32 * 0.37 + elapsed * 0.3).sin().abs() * 0.4;

            frame.draw(
                mesh,
                mat,
                Mat4::translation(x, 0.8 + (spin * 0.5).sin() * 0.3, z)
                    * Mat4::rotation_y(spin)
                    * Mat4::rotation_x(spin * 0.7)
                    * Mat4::scale(scale, scale, scale),
            );
        }

        // --- Floating shapes (upper ring) ---
        for i in 0..6 {
            let angle = (i as f32 / 6.0) * std::f32::consts::TAU + elapsed * 0.4;
            let x = angle.cos() * 2.5;
            let z = angle.sin() * 2.5;
            let y = 3.5 + (elapsed * 1.2 + i as f32).sin() * 0.5;

            let (mesh, mat) = match i % 4 {
                0 => (torus, mats.gold),
                1 => (sphere, mats.chrome),
                2 => (cylinder, mats.red_plastic),
                _ => (cube, mats.marble),
            };

            frame.draw(
                mesh,
                mat,
                Mat4::translation(x, y, z)
                    * Mat4::rotation_y(elapsed * 2.0)
                    * Mat4::rotation_z(elapsed * 1.3)
                    * Mat4::scale(0.35, 0.35, 0.35),
            );
        }

        frame
    }
}

impl WindowApp for ShowcaseApp {
    type Error = DemoError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash Deferred Rendering Showcase".to_string(),
            width: 1920,
            height: 1080,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        let (mut renderer, surface) = GpuRenderer::new_windowed(ctx.window.clone())?;

        // Upload meshes
        let sphere = renderer.create_mesh(&Mesh::sphere(1.0, 24, 48))?;
        let cube = renderer.create_mesh(&Mesh::cube(1.0))?;
        let cylinder = renderer.create_mesh(&Mesh::cylinder(0.5, 1.5, 24, 1))?;
        let torus = renderer.create_mesh(&Mesh::torus(0.6, 0.2, 24, 12))?;
        let ground = renderer.create_mesh(&Mesh::plane(20.0, 4))?;

        // Create PBR-style materials
        // Chrome: high specular, high shininess
        let chrome = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 256.0,
                specular_strength: 0.95,
            },
            color: 0xFFCC_CCCC,
            receive_light: true,
        });

        // Gold: warm color, high specular
        let gold = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 128.0,
                specular_strength: 0.8,
            },
            color: 0xFFD4_AF37,
            receive_light: true,
        });

        // Red plastic: moderate specular
        let red_plastic = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 32.0,
                specular_strength: 0.4,
            },
            color: 0xFFCC_2222,
            receive_light: true,
        });

        // Blue rubber: low specular, rough
        let blue_rubber = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 8.0,
                specular_strength: 0.1,
            },
            color: 0xFF22_44AA,
            receive_light: true,
        });

        // White marble: moderate specular
        let marble = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 64.0,
                specular_strength: 0.5,
            },
            color: 0xFFE8_E0D8,
            receive_light: true,
        });

        // Ground: dark, matte
        let ground_mat = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 4.0,
                specular_strength: 0.05,
            },
            color: 0xFF22_2222,
            receive_light: true,
        });

        self.renderer = Some(renderer);
        self.surface = Some(surface);
        self.sphere_mesh = Some(sphere);
        self.cube_mesh = Some(cube);
        self.cylinder_mesh = Some(cylinder);
        self.torus_mesh = Some(torus);
        self.ground_mesh = Some(ground);
        self.materials = Some(MaterialSet {
            chrome,
            gold,
            red_plastic,
            blue_rubber,
            marble,
            ground: ground_mat,
        });

        println!(
            "\n{}",
            "✨ Abrash Deferred Rendering Showcase".bold().cyan()
        );
        println!("{}", "=====================================".dark_grey());

        let mut info_table = Table::new();
        info_table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Property").fg(Color::Cyan),
                Cell::new("Value").fg(Color::Cyan),
            ])
            .add_row(vec![
                Cell::new("Pipeline"),
                Cell::new("Shadow > G-Buffer > Deferred Lighting > Tone Map").fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new("Lights"),
                Cell::new("1 directional (shadows) + 3 point (orbiting)").fg(Color::Yellow),
            ])
            .add_row(vec![
                Cell::new("Meshes"),
                Cell::new("sphere, cube, cylinder, torus, plane"),
            ])
            .add_row(vec![
                Cell::new("Objects"),
                Cell::new("15 objects, 6 materials"),
            ]);

        println!("\n{}", "⚙️  Info".bold());
        println!("{info_table}");

        let mut controls = Table::new();
        controls
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Input").fg(Color::Cyan),
                Cell::new("Action").fg(Color::Cyan),
            ])
            .add_row(vec![
                Cell::new("D"),
                Cell::new("Cycle debug modes (position/normal/albedo/roughness/metallic/depth)"),
            ])
            .add_row(vec![
                Cell::new("T"),
                Cell::new("Toggle TAA (temporal anti-aliasing)"),
            ])
            .add_row(vec![
                Cell::new("Space"),
                Cell::new("Pause/resume animation"),
            ])
            .add_row(vec![Cell::new("Esc"), Cell::new("Quit")]);

        println!("\n{}", "🎮 Controls".bold());
        println!("{controls}\n");

        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: &WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        if let (Some(renderer), Some(surface)) = (self.renderer.as_ref(), self.surface.as_mut()) {
            surface.resize(renderer.device(), width, height);
        }
        Ok(())
    }

    fn input(&mut self, _ctx: &WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key,
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            match logical_key {
                // D — cycle debug modes
                Key::Character(c) if c.as_str() == "d" => {
                    self.debug_mode_index = (self.debug_mode_index + 1) % DEBUG_MODES.len();
                    let mode = DEBUG_MODES[self.debug_mode_index];
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.set_debug_mode(mode);
                    }
                    println!("  Debug: {}", DEBUG_MODE_NAMES[self.debug_mode_index]);
                }
                // T — toggle TAA
                Key::Character(c) if c.as_str() == "t" => {
                    self.taa_enabled = !self.taa_enabled;
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.set_taa_enabled(self.taa_enabled);
                    }
                    println!("  TAA: {}", if self.taa_enabled { "ON" } else { "OFF" });
                }
                // Space — pause animation
                Key::Named(NamedKey::Space) => {
                    self.paused = !self.paused;
                    println!(
                        "  Animation: {}",
                        if self.paused { "PAUSED" } else { "PLAYING" }
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn update(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        if !self.paused {
            self.anim_time += dt;
        }

        let size = ctx.window.inner_size();
        let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;

        let frame = self.build_frame(self.anim_time, aspect);

        let renderer = self.renderer.as_mut().unwrap();
        let surface = self.surface.as_ref().unwrap();
        renderer
            .render_to_surface(&frame, surface)
            .map_err(DemoError::from)
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), DemoError> {
    run_windowed(ShowcaseApp::new());
    Ok(())
}
