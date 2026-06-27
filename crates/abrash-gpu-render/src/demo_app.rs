#[cfg(feature = "windowed")]
use std::sync::Arc;
#[cfg(feature = "windowed")]
use winit::{dpi::PhysicalSize, event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder};
use crate::*;


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
