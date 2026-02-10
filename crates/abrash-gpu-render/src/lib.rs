//! GPU rasterization and rendering utilities.
//!
//! This crate contains portable GPU rendering primitives and demo runtime
//! code used by the main abrash package.

use bytemuck::{Pod, Zeroable};
use std::{fmt, sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

const SHADER_SRC: &str = r#"
struct Uniforms {
    angle: f32,
    aspect: f32,
    _pad0: vec2<f32>,
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
    let cy = cos(uniforms.angle);
    let sy = sin(uniforms.angle);
    let cx = cos(uniforms.angle * 0.63);
    let sx = sin(uniforms.angle * 0.63);

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

    let world = vec3<f32>(rx.x, rx.y, rx.z - 4.5);

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
"#;

/// A GPU-ready vertex with position and linear color.
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct GpuDemoConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub rotation_speed: f32,
    pub clear_color: [f64; 4],
}

impl Default for GpuDemoConfig {
    fn default() -> Self {
        Self {
            title: "Abrash GPU Cube (wgpu)".to_string(),
            width: 1280,
            height: 720,
            rotation_speed: 0.8,
            clear_color: [0.05, 0.08, 0.12, 1.0],
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
pub fn validate_demo_config(config: &GpuDemoConfig) -> Result<(), &'static str> {
    if config.width == 0 || config.height == 0 {
        return Err("Window dimensions must be non-zero");
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VertexRaw {
    position: [f32; 3],
    color: [f32; 3],
}

impl VertexRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
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
    angle: f32,
    aspect: f32,
    _pad: [f32; 2],
}

struct GpuCubeApp {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    start_time: Instant,
    rotation_speed: f32,
    clear_color: wgpu::Color,
}

impl GpuCubeApp {
    async fn new(
        window: Arc<Window>,
        vertices: Vec<GpuVertex>,
        indices: Vec<u16>,
        demo_config: &GpuDemoConfig,
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
            .ok_or_else(|| "No suitable GPU adapter found".to_string())?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GPU Cube Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU Cube Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let uniform = SceneUniform {
            angle: 0.0,
            aspect: config.width as f32 / config.height as f32,
            _pad: [0.0, 0.0],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Uniform Layout"),
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
            label: Some("Scene Uniform Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU Cube Pipeline Layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GPU Cube Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexRaw::layout()],
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
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
        });

        let raw_vertices: Vec<VertexRaw> = vertices
            .iter()
            .map(|v| VertexRaw {
                position: v.position,
                color: v.color,
            })
            .collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(&raw_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (depth_texture, depth_view) = Self::create_depth_resources(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            depth_view,
            start_time: Instant::now(),
            rotation_speed: demo_config.rotation_speed,
            clear_color: wgpu::Color {
                r: demo_config.clear_color[0],
                g: demo_config.clear_color[1],
                b: demo_config.clear_color[2],
                a: demo_config.clear_color[3],
            },
        })
    }

    fn create_depth_resources(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
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
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        let (depth_texture, depth_view) = Self::create_depth_resources(&self.device, &self.config);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    fn update(&mut self) {
        let angle = self.start_time.elapsed().as_secs_f32() * self.rotation_speed;
        let uniform = SceneUniform {
            angle,
            aspect: self.config.width as f32 / self.config.height as f32,
            _pad: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GPU Cube Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GPU Cube Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
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

/// Runs the hardware-accelerated rotating cube demo.
pub fn run_gpu_cube() -> Result<(), String> {
    run_gpu_cube_with_config(GpuDemoConfig::default())
}

/// Runs the hardware-accelerated rotating cube demo with custom runtime settings.
pub fn run_gpu_cube_with_config(config: GpuDemoConfig) -> Result<(), String> {
    validate_demo_config(&config).map_err(str::to_string)?;
    let (vertices, indices) = unit_cube_mesh();
    validate_mesh(&vertices, &indices)
        .map_err(|err| format!("Invalid demo mesh for GPU cube: {err}"))?;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {e}"))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(config.title.as_str())
            .with_inner_size(PhysicalSize::new(config.width, config.height))
            .build(&event_loop)
            .map_err(|e| format!("Failed to create window: {e}"))?,
    );

    let mut app = pollster::block_on(GpuCubeApp::new(
        window.clone(),
        vertices,
        indices,
        &config,
    ))?;

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => app.resize(size),
                WindowEvent::RedrawRequested => {
                    app.update();
                    match app.render() {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            app.resize(app.size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            elwt.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            eprintln!("Surface timeout; skipping frame");
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        })
        .map_err(|e| format!("Event loop error: {e}"))
}
