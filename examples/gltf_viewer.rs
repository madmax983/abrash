//! glTF Viewer — loads a `.glb`/`.gltf` file and plays its skeletal animation.
//!
//! Usage: `cargo run --example gltf_viewer --features gltf -- path/to/model.glb`

use std::f32::consts::PI;
use std::path::Path;

use abrash::math::{Mat4, Vec3};
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::render_api::cpu_renderer::CpuRenderer;
use abrash::render_api::frame::{Frame, FrameCamera};
use abrash::render_api::material::Material;
use abrash::render_api::renderer::RenderError;
use abrash::render_api::target::RenderTarget;
use abrash::render_api::{MaterialHandle, MeshHandle, Renderer};
use abrash::time::FixedTimestep;

use abrash::skeletal::skinning::skin_vertices;
use abrash::skeletal::{GltfScene, SkeletonAnimator, SkinnedMesh, load_gltf};

use abrash_anim::clock::PlaybackMode;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF1A_1A2E;
const TITLE: &str = "Abrash - glTF Viewer";

// ---------------------------------------------------------------------------
// AABB helper
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box.
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    fn from_positions(positions: &[Vec3]) -> Self {
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        for p in positions {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
        Self { min, max }
    }

    fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    fn extent(&self) -> f32 {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        dx.max(dy).max(dz)
    }
}

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

struct GltfViewerApp {
    presenter: Option<SoftwarePresenter>,
    renderer: CpuRenderer,
    target: RenderTarget,
    timestep: FixedTimestep,

    // Animation state
    animator: Option<SkeletonAnimator>,
    skinned_mesh: Option<SkinnedMesh>,
    mesh_handle: Option<MeshHandle>,
    material_handle: Option<MaterialHandle>,
    skinned_positions: Vec<Vec3>,

    // Camera orbit
    camera_angle: f32,
    model_center: Vec3,
    camera_distance: f32,
}

impl GltfViewerApp {
    fn new(scene: GltfScene) -> Result<Self, HostError> {
        let mut renderer = CpuRenderer::new(WIDTH, HEIGHT);
        let target = RenderTarget::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?;

        // Take first mesh (if any)
        let skinned_mesh = scene.meshes.into_iter().next();

        let (mesh_handle, material_handle, skinned_positions, model_center, camera_distance) =
            if let Some(ref sm) = skinned_mesh {
                // Upload the initial mesh
                let mh = renderer
                    .create_mesh(&sm.mesh)
                    .map_err(|e| HostError::App(e.to_string()))?;

                // Build material from glTF data or fall back to flat gray
                let mat = if scene.materials.is_empty() {
                    Material::flat(0xFFA0_A0A0)
                } else {
                    let gltf_mat = &scene.materials[0];
                    let factor = gltf_mat.base_color_factor;
                    let r = (factor[0] * 255.0) as u32;
                    let g = (factor[1] * 255.0) as u32;
                    let b = (factor[2] * 255.0) as u32;
                    let a = (factor[3] * 255.0) as u32;
                    let color = (a << 24) | (r << 16) | (g << 8) | b;
                    Material::flat(color)
                };
                let math = renderer
                    .create_material(mat)
                    .map_err(|e| HostError::App(e.to_string()))?;

                let positions = sm.mesh.vertices.clone();

                // Compute AABB for camera placement
                let aabb = Aabb::from_positions(&sm.mesh.vertices);
                let center = aabb.center();
                let extent = aabb.extent().max(0.1);
                // Place camera far enough to see the whole model
                let dist = extent * 1.5;

                (Some(mh), Some(math), positions, center, dist)
            } else {
                (None, None, Vec::new(), Vec3::ZERO, 5.0)
            };

        // Build animator from first clip (if skeleton + clips exist)
        let animator = scene.skeleton.and_then(|skel| {
            scene
                .clips
                .first()
                .map(|clip| SkeletonAnimator::new(skel, clip, PlaybackMode::Loop))
        });

        Ok(Self {
            presenter: None,
            renderer,
            target,
            timestep: FixedTimestep::new(60),
            animator,
            skinned_mesh,
            mesh_handle,
            material_handle,
            skinned_positions,
            camera_angle: 0.0,
            model_center,
            camera_distance,
        })
    }
}

impl WindowApp for GltfViewerApp {
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

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.renderer = CpuRenderer::new(width, height);
        self.target =
            RenderTarget::new(width, height).map_err(|e| HostError::App(e.to_string()))?;

        // Re-upload mesh and material after renderer reset
        if let Some(ref sm) = self.skinned_mesh {
            self.mesh_handle = Some(
                self.renderer
                    .create_mesh(&sm.mesh)
                    .map_err(|e| HostError::App(e.to_string()))?,
            );
            let mat = Material::flat(0xFFA0_A0A0);
            self.material_handle = Some(
                self.renderer
                    .create_material(mat)
                    .map_err(|e| HostError::App(e.to_string()))?,
            );
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        let dt = self.timestep.dt();

        for _ in 0..steps {
            // Orbit camera
            self.camera_angle += dt * 0.5;

            // Tick animation and skin vertices
            if let (Some(animator), Some(sm)) = (&mut self.animator, &self.skinned_mesh) {
                // Tick the animation clock forward.
                animator.tick(dt);
                // Retrieve the updated pose and the skeleton immutably at the same time.
                let pose = animator.current_pose();
                let skeleton = animator.skeleton();
                let globals = skeleton.compute_global_transforms(pose);
                let skin_mats = skeleton.compute_skin_matrices(&globals);
                skin_vertices(sm, &skin_mats, &mut self.skinned_positions);

                // Build an updated mesh with the skinned positions
                if let Some(mh) = self.mesh_handle {
                    let mut updated_mesh = sm.mesh.clone();
                    updated_mesh.vertices.clone_from(&self.skinned_positions);
                    self.renderer
                        .update_mesh(mh, &updated_mesh)
                        .map_err(render_err_to_host)?;
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let (w, h) = (self.target.width() as f32, self.target.height() as f32);
        let aspect = w / h;

        let projection = Mat4::perspective(PI / 3.0, aspect, 0.01, 1000.0);

        let cx = self.model_center.x + self.camera_distance * self.camera_angle.cos();
        let cz = self.model_center.z + self.camera_distance * self.camera_angle.sin();
        let eye = Vec3::new(cx, self.model_center.y + self.camera_distance * 0.3, cz);

        let view = Mat4::look_at(eye, self.model_center, Vec3::new(0.0, 1.0, 0.0));

        let camera = FrameCamera::new(view, projection);
        let mut frame = Frame::new(camera);
        frame.clear_color = Some(BACKGROUND);

        if let (Some(mh), Some(math)) = (self.mesh_handle, self.material_handle) {
            frame.draw(mh, math, Mat4::identity());
        }

        self.renderer
            .render_frame(&frame, &mut self.target)
            .map_err(render_err_to_host)?;

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(self.target.framebuffer())?;
        Ok(())
    }
}

#[allow(clippy::needless_pass_by_value)] // Required for map_err function pointer
fn render_err_to_host(e: RenderError) -> HostError {
    HostError::App(e.to_string())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), HostError> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gltf_viewer <path/to/model.glb>");
        eprintln!();
        eprintln!("Loads a glTF 2.0 file and plays its first animation clip.");
        eprintln!("The camera orbits the model automatically.");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    if !path.exists() {
        eprintln!("Error: file not found: {}", path.display());
        std::process::exit(1);
    }

    println!("Loading: {}", path.display());
    let scene = load_gltf(path).map_err(|e| HostError::App(e.to_string()))?;
    println!(
        "Loaded: {} meshes, {} clips, {} textures, {} materials",
        scene.meshes.len(),
        scene.clips.len(),
        scene.textures.len(),
        scene.materials.len(),
    );
    if let Some(ref skel) = scene.skeleton {
        println!("Skeleton: {} joints", skel.joint_count());
    }

    let app = GltfViewerApp::new(scene)?;
    run_windowed(app)
}
