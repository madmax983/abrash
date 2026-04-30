//! GPU rasterization and rendering utilities.
//!
//! This crate contains portable GPU rendering primitives and demo runtime
//! code used by the main abrash package.

#[cfg(feature = "ray-tracing")]
pub mod accel_structure;
pub mod blitter;
pub mod capture;
pub mod composition;
pub mod deferred;
pub mod device;
pub mod environment;
pub mod gbuffer;
pub mod ibl;
pub mod mesh_buffer;
pub mod postprocess;
#[cfg(feature = "ray-tracing")]
pub mod raytracing;
pub mod renderer;
#[cfg(feature = "ray-tracing")]
pub mod rt_reflections;
pub mod shader;
pub mod shadow;
#[cfg(feature = "windowed")]
pub mod surface;
pub mod svgf;
pub mod taa;
pub mod temporal;

use bytemuck::{Pod, Zeroable};
use std::fmt;
#[cfg(feature = "windowed")]
use std::{sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
#[cfg(feature = "windowed")]
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const SHADER_SRC: &str = r"
struct Uniforms {
    yaw: f32,
    pitch: f32,
    aspect: f32,
    distance: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    let cy = cos(uniforms.yaw);
    let sy = sin(uniforms.yaw);
    let cx = cos(uniforms.pitch);
    let sx = sin(uniforms.pitch);

    let ry = vec3<f32>(
        input.position.x * cy + input.position.z * sy,
        input.position.y,
        -input.position.x * sy + input.position.z * cy
    );

    let rx = vec3<f32>(
        ry.x,
        ry.y * cx - ry.z * sx,
        ry.y * sx + ry.z * cx
    );

    let world = vec3<f32>(rx.x, rx.y, rx.z - uniforms.distance);

    let fov = 1.0;
    let f = 1.0 / tan(fov * 0.5);
    let near = 0.1;
    let far = 100.0;

    let proj = mat4x4<f32>(
        vec4<f32>(f / uniforms.aspect, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, f, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, far / (near - far), -1.0),
        vec4<f32>(0.0, 0.0, (near * far) / (near - far), 0.0)
    );

    var out: VsOut;
    out.position = proj * vec4<f32>(world, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
";

/// A GPU-ready vertex with position and linear color.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

/// A single triangle for GPU rasterization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuTriangle {
    pub vertices: [GpuVertex; 3],
}

/// Runtime configuration for the GPU demo window and animation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct GpuDemoConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub enable_keyboard_input: bool,
    pub enable_mouse_input: bool,
    pub auto_rotate: bool,
    pub rotation_speed: f32,
    pub key_rotation_speed: f32,
    pub key_zoom_speed: f32,
    pub mouse_sensitivity: f32,
    pub initial_yaw: f32,
    pub initial_pitch: f32,
    pub initial_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub clear_color: [f64; 4],
}

impl Default for GpuDemoConfig {
    fn default() -> Self {
        Self {
            title: "Abrash GPU Cube (wgpu)".to_string(),
            width: 1280,
            height: 720,
            enable_keyboard_input: true,
            enable_mouse_input: true,
            auto_rotate: true,
            rotation_speed: 0.8,
            key_rotation_speed: 1.6,
            key_zoom_speed: 3.0,
            mouse_sensitivity: 0.01,
            initial_yaw: 0.0,
            initial_pitch: 0.35,
            initial_distance: 4.5,
            min_distance: 1.5,
            max_distance: 20.0,
            clear_color: [0.05, 0.08, 0.12, 1.0],
        }
    }
}

/// Stateful keyboard/mouse interaction controller for a mesh demo camera.
#[cfg(feature = "windowed")]
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct GpuInteractionController {
    yaw: f32,
    pitch: f32,
    distance: f32,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    zoom_in: bool,
    zoom_out: bool,
    drag_active: bool,
    last_cursor: Option<(f64, f64)>,
    auto_rotate_enabled: bool,
}

#[cfg(feature = "windowed")]
impl GpuInteractionController {
    #[must_use]
    pub const fn new(config: &GpuDemoConfig) -> Self {
        Self {
            yaw: config.initial_yaw,
            pitch: config.initial_pitch,
            distance: config.initial_distance,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
            zoom_in: false,
            zoom_out: false,
            drag_active: false,
            last_cursor: None,
            auto_rotate_enabled: config.auto_rotate,
        }
    }

    #[must_use]
    pub const fn yaw_radians(&self) -> f32 {
        self.yaw
    }

    #[must_use]
    pub const fn pitch_radians(&self) -> f32 {
        self.pitch
    }

    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub const fn set_move_left(&mut self, pressed: bool) {
        self.move_left = pressed;
    }

    pub const fn set_move_right(&mut self, pressed: bool) {
        self.move_right = pressed;
    }

    pub const fn set_move_up(&mut self, pressed: bool) {
        self.move_up = pressed;
    }

    pub const fn set_move_down(&mut self, pressed: bool) {
        self.move_down = pressed;
    }

    pub fn adjust_zoom(&mut self, delta: f32, config: &GpuDemoConfig) {
        self.distance = (self.distance + delta).clamp(config.min_distance, config.max_distance);
    }

    pub const fn reset(&mut self, config: &GpuDemoConfig) {
        self.yaw = config.initial_yaw;
        self.pitch = config.initial_pitch;
        self.distance = config
            .initial_distance
            .clamp(config.min_distance, config.max_distance);
    }

    pub const fn toggle_auto_rotate(&mut self) {
        self.auto_rotate_enabled = !self.auto_rotate_enabled;
    }

    pub fn update(&mut self, dt_seconds: f32, config: &GpuDemoConfig) {
        if self.auto_rotate_enabled {
            self.yaw += config.rotation_speed * dt_seconds;
        }

        if config.enable_keyboard_input {
            if self.move_left {
                self.yaw -= config.key_rotation_speed * dt_seconds;
            }
            if self.move_right {
                self.yaw += config.key_rotation_speed * dt_seconds;
            }
            if self.move_up {
                self.pitch += config.key_rotation_speed * dt_seconds;
            }
            if self.move_down {
                self.pitch -= config.key_rotation_speed * dt_seconds;
            }
            if self.zoom_in {
                self.adjust_zoom(-config.key_zoom_speed * dt_seconds, config);
            }
            if self.zoom_out {
                self.adjust_zoom(config.key_zoom_speed * dt_seconds, config);
            }
        }

        self.pitch = self.pitch.clamp(-1.45, 1.45);
        self.distance = self
            .distance
            .clamp(config.min_distance, config.max_distance);
    }

    pub fn apply_mouse_drag(&mut self, dx: f32, dy: f32, config: &GpuDemoConfig) {
        self.yaw += dx * config.mouse_sensitivity;
        self.pitch -= dy * config.mouse_sensitivity;
        self.pitch = self.pitch.clamp(-1.45, 1.45);
    }

    #[cfg(feature = "windowed")]
    pub fn handle_window_event(&mut self, event: &WindowEvent, config: &GpuDemoConfig) {
        match event {
            WindowEvent::KeyboardInput { event, .. } if config.enable_keyboard_input => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::ArrowLeft | KeyCode::KeyA => self.set_move_left(pressed),
                        KeyCode::ArrowRight | KeyCode::KeyD => self.set_move_right(pressed),
                        KeyCode::ArrowUp | KeyCode::KeyW => self.set_move_up(pressed),
                        KeyCode::ArrowDown | KeyCode::KeyS => self.set_move_down(pressed),
                        KeyCode::KeyQ => self.zoom_in = pressed,
                        KeyCode::KeyE => self.zoom_out = pressed,
                        KeyCode::Space if pressed => self.toggle_auto_rotate(),
                        KeyCode::KeyR if pressed => self.reset(config),
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if config.enable_mouse_input => {
                self.drag_active = *state == ElementState::Pressed;
                if !self.drag_active {
                    self.last_cursor = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } if config.enable_mouse_input => {
                if self.drag_active {
                    if let Some((last_x, last_y)) = self.last_cursor {
                        self.apply_mouse_drag(
                            (position.x - last_x) as f32,
                            (position.y - last_y) as f32,
                            config,
                        );
                    }
                    self.last_cursor = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } if config.enable_mouse_input => {
                let zoom_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -(*y) * config.key_zoom_speed * 0.2,
                    MouseScrollDelta::PixelDelta(px) => {
                        -(px.y as f32) * config.key_zoom_speed * 0.01
                    }
                };
                self.adjust_zoom(zoom_delta, config);
            }
            _ => {}
        }
    }
}

/// Validation errors for indexed triangle meshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshValidationError {
    EmptyVertices,
    EmptyIndices,
    IndexCountNotMultipleOf3 { index_count: usize },
    IndexOutOfBounds { index: u16, vertex_count: usize },
}

impl fmt::Display for MeshValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVertices => write!(f, "mesh must contain at least one vertex"),
            Self::EmptyIndices => write!(f, "mesh must contain at least one index"),
            Self::IndexCountNotMultipleOf3 { index_count } => {
                write!(f, "index count ({index_count}) must be a multiple of 3")
            }
            Self::IndexOutOfBounds {
                index,
                vertex_count,
            } => {
                write!(
                    f,
                    "index {index} is out of bounds for vertex count {vertex_count}"
                )
            }
        }
    }
}

/// Validates that a mesh is a non-empty indexed triangle list.
///
/// # Errors
///
/// Returns a [`MeshValidationError`] when the slice data is empty, not a
/// multiple of three, or references an out-of-bounds vertex.
pub fn validate_mesh(vertices: &[GpuVertex], indices: &[u16]) -> Result<(), MeshValidationError> {
    if vertices.is_empty() {
        return Err(MeshValidationError::EmptyVertices);
    }
    if indices.is_empty() {
        return Err(MeshValidationError::EmptyIndices);
    }
    if indices.len() % 3 != 0 {
        return Err(MeshValidationError::IndexCountNotMultipleOf3 {
            index_count: indices.len(),
        });
    }
    for &index in indices {
        if usize::from(index) >= vertices.len() {
            return Err(MeshValidationError::IndexOutOfBounds {
                index,
                vertex_count: vertices.len(),
            });
        }
    }
    Ok(())
}

/// Validates that demo configuration has a drawable window size.
///
/// # Errors
///
/// Returns an error if window dimensions or camera distance limits are invalid.
pub fn validate_demo_config(config: &GpuDemoConfig) -> Result<(), &'static str> {
    if config.width == 0 || config.height == 0 {
        return Err("Window dimensions must be non-zero");
    }
    if config.min_distance <= 0.0 || config.max_distance <= 0.0 {
        return Err("Camera distance limits must be positive");
    }
    if config.min_distance > config.max_distance {
        return Err("min_distance must be less than or equal to max_distance");
    }
    if config.initial_distance < config.min_distance
        || config.initial_distance > config.max_distance
    {
        return Err("initial_distance must be within [min_distance, max_distance]");
    }
    Ok(())
}

/// Returns a colored unit cube mesh (8 vertices, 36 indices).
#[must_use]
pub fn unit_cube_mesh() -> (Vec<GpuVertex>, Vec<u16>) {
    let vertices = vec![
        GpuVertex {
            position: [-1.0, -1.0, -1.0],
            color: [1.0, 0.2, 0.2],
        },
        GpuVertex {
            position: [1.0, -1.0, -1.0],
            color: [0.2, 1.0, 0.2],
        },
        GpuVertex {
            position: [1.0, 1.0, -1.0],
            color: [0.2, 0.2, 1.0],
        },
        GpuVertex {
            position: [-1.0, 1.0, -1.0],
            color: [1.0, 1.0, 0.2],
        },
        GpuVertex {
            position: [-1.0, -1.0, 1.0],
            color: [1.0, 0.2, 1.0],
        },
        GpuVertex {
            position: [1.0, -1.0, 1.0],
            color: [0.2, 1.0, 1.0],
        },
        GpuVertex {
            position: [1.0, 1.0, 1.0],
            color: [1.0, 0.7, 0.2],
        },
        GpuVertex {
            position: [-1.0, 1.0, 1.0],
            color: [0.8, 0.8, 0.8],
        },
    ];

    let indices: Vec<u16> = vec![
        // Back face
        0, 1, 2, 2, 3, 0, // Front face
        4, 6, 5, 6, 4, 7, // Left face
        4, 0, 3, 3, 7, 4, // Right face
        1, 5, 6, 6, 2, 1, // Bottom face
        4, 5, 1, 1, 0, 4, // Top face
        3, 2, 6, 6, 7, 3,
    ];

    (vertices, indices)
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    yaw: f32,
    pitch: f32,
    aspect: f32,
    distance: f32,
}

#[cfg(feature = "windowed")]
struct GpuMeshApp {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    last_update: Instant,
    demo_config: GpuDemoConfig,
    interaction: GpuInteractionController,
    clear_color: wgpu::Color,
}

#[cfg(feature = "windowed")]
impl GpuMeshApp {
    /// # Errors
    ///
    /// Returns an error if the window, adapter, device, or surface cannot be
    /// created.
    /// # Panics
    ///
    /// Panics if the selected surface reports no supported formats or present
    /// modes.
    #[allow(clippy::too_many_lines)]
    async fn new(
        window: Arc<Window>,
        vertices: Vec<GpuVertex>,
        indices: Vec<u16>,
        demo_config: GpuDemoConfig,
    ) -> Result<Self, String> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err("Window size must be non-zero".to_string());
        }

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|e| format!("Failed to create surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("No suitable GPU adapter found: {e:?}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("GPU Cube Device "),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                #[allow(clippy::default_trait_access)]
                experimental_features: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("Failed to create device: {e}"))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);

        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(capabilities.present_modes[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU Cube Shader "),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let interaction = GpuInteractionController::new(&demo_config);
        let uniform = SceneUniform {
            yaw: interaction.yaw_radians(),
            pitch: interaction.pitch_radians(),
            aspect: surface_config.width as f32 / surface_config.height as f32,
            distance: interaction.distance(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniform Buffer "),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Uniform Layout "),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Uniform Bind Group "),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU Cube Pipeline Layout "),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GPU Cube Pipeline "),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[GpuVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ⚡ Bolt: Eliminate O(N) intermediate heap allocation by using bytemuck to directly cast GpuVertex slice to bytes
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer  "),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer "),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (depth_texture, depth_view) = Self::create_depth_resources(&device, &surface_config);
        let clear_color = wgpu::Color {
            r: demo_config.clear_color[0],
            g: demo_config.clear_color[1],
            b: demo_config.clear_color[2],
            a: demo_config.clear_color[3],
        };

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            size,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            depth_view,
            last_update: Instant::now(),
            interaction,
            demo_config,
            clear_color,
        })
    }

    fn create_depth_resources(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture "),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (depth_texture, depth_view)
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        let (depth_texture, depth_view) =
            Self::create_depth_resources(&self.device, &self.surface_config);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        self.interaction
            .handle_window_event(event, &self.demo_config);
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();
        self.last_update = now;
        self.interaction.update(dt, &self.demo_config);

        let uniform = SceneUniform {
            yaw: self.interaction.yaw_radians(),
            pitch: self.interaction.pitch_radians(),
            aspect: self.surface_config.width as f32 / self.surface_config.height as f32,
            distance: self.interaction.distance(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn render(&self) -> Result<(), String> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            e => return Err(format!("surface unavailable: {e:?}")),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GPU Cube Encoder "),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GPU Cube Render Pass "),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

/// Runs a hardware-accelerated mesh demo with camera controls.
///
/// # Errors
///
/// Returns an error if mesh validation fails or the event loop/window/GPU setup
/// cannot be created.
#[allow(clippy::too_many_lines)]
#[cfg(feature = "windowed")]
pub fn run_mesh_demo(
    vertices: Vec<GpuVertex>,
    indices: Vec<u16>,
    config: GpuDemoConfig,
) -> Result<(), String> {
    validate_demo_config(&config).map_err(str::to_string)?;
    validate_mesh(&vertices, &indices)
        .map_err(|err| format!("Invalid mesh for GPU render demo: {err}"))?;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {e}"))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(config.title.as_str())
            .with_inner_size(PhysicalSize::new(config.width, config.height))
            .build(&event_loop)
            .map_err(|e| format!("Failed to create window: {e}"))?,
    );

    let mut app = pollster::block_on(GpuMeshApp::new(window.clone(), vertices, indices, config))?;

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                app.handle_window_event(&event);
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => app.resize(size),
                    WindowEvent::RedrawRequested => {
                        app.update();
                        match app.render() {
                            Ok(()) => {}
                            Err(ref e) if e.contains("Lost") || e.contains("Outdated") => {
                                app.resize(app.size);
                            }
                            Err(ref e) if e.contains("OutOfMemory") => {
                                elwt.exit();
                            }
                            Err(ref e) => {
                                eprintln!("Render error: {e}; skipping frame");
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        })
        .map_err(|e| format!("Event loop error: {e}"))
}

/// Runs the hardware-accelerated rotating cube demo.
///
/// # Errors
///
/// Returns an error if the default demo configuration or GPU/window setup
/// cannot be created.
#[cfg(feature = "windowed")]
pub fn run_gpu_cube() -> Result<(), String> {
    run_gpu_cube_with_config(GpuDemoConfig::default())
}

/// Runs the hardware-accelerated rotating cube demo with custom runtime settings.
///
/// # Errors
///
/// Returns an error if the provided configuration or GPU/window setup cannot be
/// created.
#[cfg(feature = "windowed")]
pub fn run_gpu_cube_with_config(config: GpuDemoConfig) -> Result<(), String> {
    let (vertices, indices) = unit_cube_mesh();
    run_mesh_demo(vertices, indices, config)
}

/// Configuration for offscreen GPU renderer benchmarks.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuOffscreenBenchConfig {
    pub width: u32,
    pub height: u32,
    pub initial_yaw: f32,
    pub initial_pitch: f32,
    pub initial_distance: f32,
    pub rotation_speed: f32,
    pub draw_repeats: u32,
    pub clear_color: [f64; 4],
}

impl Default for GpuOffscreenBenchConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            initial_yaw: 0.0,
            initial_pitch: 0.35,
            initial_distance: 4.5,
            rotation_speed: 0.8,
            draw_repeats: 1,
            clear_color: [0.05, 0.08, 0.12, 1.0],
        }
    }
}

/// Offscreen GPU renderer suitable for repeatable frame benchmarks.
pub struct GpuOffscreenBench {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    _color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    config: GpuOffscreenBenchConfig,
    yaw: f32,
    clear_color: wgpu::Color,
}

impl GpuOffscreenBench {
    /// Creates an offscreen benchmark renderer for an indexed mesh.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh or benchmark configuration is invalid, or if
    /// the GPU device cannot be created.
    ///
    /// # Panics
    ///
    /// Panics if the adapter exposes no supported color formats.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        vertices: &[GpuVertex],
        indices: &[u16],
        config: GpuOffscreenBenchConfig,
    ) -> Result<Self, String> {
        if config.width == 0 || config.height == 0 {
            return Err("Offscreen benchmark dimensions must be non-zero".to_string());
        }
        if config.draw_repeats == 0 {
            return Err("Offscreen benchmark draw_repeats must be >= 1".to_string());
        }
        validate_mesh(vertices, indices).map_err(|err| format!("Invalid benchmark mesh: {err}"))?;

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("No suitable GPU adapter found for offscreen benchmark: {e:?}"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Offscreen GPU Bench Device "),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            #[allow(clippy::default_trait_access)]
            experimental_features: Default::default(),
            trace: wgpu::Trace::Off,
        }))
        .map_err(|e| format!("Failed to create offscreen benchmark device: {e}"))?;

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Bench Color "),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Bench Depth "),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Offscreen GPU Bench Shader "),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let uniform = SceneUniform {
            yaw: config.initial_yaw,
            pitch: config.initial_pitch,
            aspect: config.width as f32 / config.height as f32,
            distance: config.initial_distance,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offscreen Bench Uniform Buffer "),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Offscreen Bench Uniform Layout "),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Offscreen Bench Uniform Bind Group "),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Offscreen Bench Pipeline Layout "),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Offscreen Bench Pipeline "),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[GpuVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ⚡ Bolt: Eliminate O(N) intermediate heap allocation by using bytemuck to directly cast GpuVertex slice to bytes
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offscreen Bench Vertex Buffer  "),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offscreen Bench Index Buffer "),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            _color_texture: color_texture,
            color_view,
            _depth_texture: depth_texture,
            depth_view,
            yaw: config.initial_yaw,
            clear_color: wgpu::Color {
                r: config.clear_color[0],
                g: config.clear_color[1],
                b: config.clear_color[2],
                a: config.clear_color[3],
            },
            config,
        })
    }

    /// Renders a single offscreen frame and blocks until GPU completion.
    ///
    /// # Errors
    ///
    /// Returns an error if command submission or GPU completion fails.
    pub fn render_frame(&mut self) -> Result<(), String> {
        self.yaw += self.config.rotation_speed * (1.0 / 60.0);
        let uniform = SceneUniform {
            yaw: self.yaw,
            pitch: self.config.initial_pitch,
            aspect: self.config.width as f32 / self.config.height as f32,
            distance: self.config.initial_distance,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Offscreen Bench Encoder "),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Offscreen Bench Pass "),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        // No readback in the benchmark path — skip the writeback.
                        store: wgpu::StoreOp::Discard,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        // Depth is consumed in-pass; discard saves the writeback.
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            for _ in 0..self.config.draw_repeats {
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        Ok(())
    }

    /// Renders multiple offscreen frames and blocks for completion each frame.
    ///
    /// # Errors
    ///
    /// Returns an error from the first frame that fails to render.
    pub fn render_frames(&mut self, frame_count: u32) -> Result<(), String> {
        for _ in 0..frame_count {
            self.render_frame()?;
        }
        Ok(())
    }

    /// Returns benchmark render target dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}
