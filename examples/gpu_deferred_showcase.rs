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
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_gpu_render::surface::GpuSurface;
use abrash_render::render_api::frame::{DirectionalLight, Frame, FrameCamera, Light, PointLight};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
use abrash_render::render_api::material::{Material, ShadingMode};
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

struct ShowcaseApp {
    renderer: Option<GpuRenderer>,
    surface: Option<GpuSurface>,
    sphere_mesh: Option<MeshHandle>,
    cube_mesh: Option<MeshHandle>,
    ground_mesh: Option<MeshHandle>,
    materials: Option<MaterialSet>,
    start: Instant,
}

impl ShowcaseApp {
    fn new() -> Self {
        Self {
            renderer: None,
            surface: None,
            sphere_mesh: None,
            cube_mesh: None,
            ground_mesh: None,
            materials: None,
            start: Instant::now(),
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

        let mut frame = Frame::new(camera);
        frame.clear_color = Some(0xFF0A0A12); // dark blue-black

        // --- Lights ---

        // Warm directional sun (casts shadows)
        frame.add_light(Light::Directional(DirectionalLight {
            direction: Vec3::new(0.4, -0.8, -0.3),
            color: 0xFFF5E6C8, // warm white
            intensity: 1.2,
        }));

        // Red point light (orbits)
        let red_angle = elapsed * 1.5;
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(red_angle.cos() * 4.0, 2.5, red_angle.sin() * 4.0),
            color: 0xFFFF3333,
            intensity: 3.0,
            radius: 8.0,
        }));

        // Blue point light (orbits opposite)
        let blue_angle = elapsed * 1.5 + std::f32::consts::PI;
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(blue_angle.cos() * 3.5, 1.5, blue_angle.sin() * 3.5),
            color: 0xFF3366FF,
            intensity: 2.5,
            radius: 7.0,
        }));

        // Green point light (static, above)
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(0.0, 5.0, 0.0),
            color: 0xFF33FF66,
            intensity: 1.5,
            radius: 12.0,
        }));

        let mats = self.materials.as_ref().unwrap();
        let sphere = self.sphere_mesh.unwrap();
        let cube = self.cube_mesh.unwrap();
        let ground = self.ground_mesh.unwrap();

        // --- Ground plane ---
        frame.draw(
            ground,
            mats.ground,
            Mat4::translation(0.0, -0.5, 0.0) * Mat4::scale(20.0, 0.1, 20.0),
        );

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

            let (mesh, mat) = match i % 5 {
                0 => (cube, mats.gold),
                1 => (sphere, mats.red_plastic),
                2 => (cube, mats.blue_rubber),
                3 => (sphere, mats.marble),
                _ => (cube, mats.chrome),
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

        // --- Floating cubes (upper ring) ---
        for i in 0..5 {
            let angle = (i as f32 / 5.0) * std::f32::consts::TAU + elapsed * 0.4;
            let x = angle.cos() * 2.5;
            let z = angle.sin() * 2.5;
            let y = 3.5 + (elapsed * 1.2 + i as f32).sin() * 0.5;

            let mat = match i % 3 {
                0 => mats.gold,
                1 => mats.chrome,
                _ => mats.red_plastic,
            };

            frame.draw(
                cube,
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

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let (mut renderer, surface) = GpuRenderer::new_windowed(ctx.window)?;

        // Upload meshes
        let sphere = renderer.create_mesh(&Mesh::cube(1.0))?; // cube as sphere stand-in
        let cube = renderer.create_mesh(&Mesh::cube(1.0))?;

        // Ground plane (flat cube)
        let ground = renderer.create_mesh(&Mesh::cube(1.0))?;

        // Create PBR-style materials
        // Chrome: high specular, high shininess
        let chrome = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 256.0,
                specular_strength: 0.95,
            },
            color: 0xFFCCCCCC,
            receive_light: true,
        });

        // Gold: warm color, high specular
        let gold = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 128.0,
                specular_strength: 0.8,
            },
            color: 0xFFD4AF37,
            receive_light: true,
        });

        // Red plastic: moderate specular
        let red_plastic = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 32.0,
                specular_strength: 0.4,
            },
            color: 0xFFCC2222,
            receive_light: true,
        });

        // Blue rubber: low specular, rough
        let blue_rubber = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 8.0,
                specular_strength: 0.1,
            },
            color: 0xFF2244AA,
            receive_light: true,
        });

        // White marble: moderate specular
        let marble = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 64.0,
                specular_strength: 0.5,
            },
            color: 0xFFE8E0D8,
            receive_light: true,
        });

        // Ground: dark, matte
        let ground_mat = renderer.create_material(Material {
            shading: ShadingMode::Phong {
                shininess: 4.0,
                specular_strength: 0.05,
            },
            color: 0xFF222222,
            receive_light: true,
        });

        self.renderer = Some(renderer);
        self.surface = Some(surface);
        self.sphere_mesh = Some(sphere);
        self.cube_mesh = Some(cube);
        self.ground_mesh = Some(ground);
        self.materials = Some(MaterialSet {
            chrome,
            gold,
            red_plastic,
            blue_rubber,
            marble,
            ground: ground_mat,
        });

        println!("\n  Abrash Deferred Rendering Showcase");
        println!("  ===================================");
        println!("  Pipeline: Shadow > G-Buffer > Deferred Lighting > Tone Map");
        println!("  Lights: 1 directional (shadows) + 3 point (orbiting)");
        println!("  Objects: 14 objects, 6 materials\n");

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
        let elapsed = self.start.elapsed().as_secs_f32();
        let size = ctx.window.inner_size();
        let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;

        let frame = self.build_frame(elapsed, aspect);

        let renderer = self.renderer.as_mut().unwrap();
        let surface = self.surface.as_ref().unwrap();
        renderer
            .render_to_surface(&frame, surface)
            .map_err(DemoError::from)
    }
}

fn main() -> Result<(), abrash::platform::HostError> {
    run_windowed(ShowcaseApp::new())
}
