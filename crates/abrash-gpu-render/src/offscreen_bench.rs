use crate::*;


use wgpu::util::DeviceExt;
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
