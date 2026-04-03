//! Ray-traced shadow pass using hardware ray queries.
//!
//! Replaces the shadow map with per-pixel ray-traced shadows when RT hardware
//! is available. Traces one shadow ray per pixel from the G-Buffer world position
//! toward the directional light, outputting a shadow factor (0 = shadowed, 1 = lit).
//!
//! Requires `wgpu::Features::EXPERIMENTAL_RAY_QUERY`.

/// WGSL compute shader for ray-traced shadows.
///
/// Reads G-Buffer position texture, traces shadow rays against the TLAS,
/// writes shadow factor to a storage texture.
const RT_SHADOW_SHADER: &str = r"
enable wgpu_ray_query;

struct LightData {
    direction: vec3<f32>,
    max_distance: f32,
};

@group(0) @binding(0) var gbuf_position: texture_2d<f32>;
@group(0) @binding(1) var<uniform> light: LightData;
@group(0) @binding(2) var scene_tlas: acceleration_structure;
@group(0) @binding(3) var output: texture_storage_2d<r8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(gbuf_position);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let pos = textureLoad(gbuf_position, gid.xy, 0);

    // Skip background pixels
    if pos.w < 0.5 {
        textureStore(output, gid.xy, vec4(1.0));
        return;
    }

    let world_pos = pos.xyz;
    let light_dir = normalize(-light.direction);

    // Trace shadow ray: from surface toward light
    var rq: ray_query;
    let ray_flags = 0u; // RAY_FLAG_NONE
    let cull_mask = 0xFFu;

    rayQueryInitialize(
        &rq,
        scene_tlas,
        ray_flags,
        cull_mask,
        world_pos + light_dir * 0.01, // bias to avoid self-intersection
        0.0,                           // t_min
        light_dir,
        light.max_distance,            // t_max
    );

    rayQueryProceed(&rq);

    let intersection = rayQueryGetCommittedIntersection(&rq);

    var shadow: f32 = 1.0;
    if intersection.kind != RAY_QUERY_INTERSECTION_NONE {
        shadow = 0.0; // hit something → in shadow
    }

    textureStore(output, gid.xy, vec4(shadow, 0.0, 0.0, 1.0));
}
";

/// Light direction uniform for the RT shadow pass.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RtShadowLightData {
    pub direction: [f32; 3],
    pub max_distance: f32,
}

/// Ray-traced shadow pass: traces shadow rays from G-Buffer positions.
pub struct RtShadowPass {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) light_buffer: wgpu::Buffer,
    /// Output shadow factor texture (R8Unorm, same size as G-Buffer).
    pub(crate) shadow_texture: Option<wgpu::Texture>,
    pub(crate) shadow_view: Option<wgpu::TextureView>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl RtShadowPass {
    /// Create the RT shadow pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RT Shadow Compute"),
            source: wgpu::ShaderSource::Wgsl(RT_SHADOW_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RT Shadow Layout"),
            entries: &[
                // G-Buffer position
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Light direction
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            RtShadowLightData,
                        >()
                            as u64),
                    },
                    count: None,
                },
                // TLAS
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::AccelerationStructure {
                        vertex_return: false,
                    },
                    count: None,
                },
                // Shadow output
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::R8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RT Shadow Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RT Shadow Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RT Shadow Light"),
            contents: bytemuck::bytes_of(&RtShadowLightData {
                direction: [0.0, -1.0, 0.0],
                max_distance: 200.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            light_buffer,
            shadow_texture: None,
            shadow_view: None,
            width: 0,
            height: 0,
        }
    }

    /// Ensure the shadow output texture matches the given dimensions.
    pub fn ensure_output(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && self.shadow_texture.is_some() {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RT Shadow Output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.shadow_texture = Some(texture);
        self.shadow_view = Some(view);
        self.width = width;
        self.height = height;
    }

    /// Encode the RT shadow compute pass.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        gbuffer_position_view: &wgpu::TextureView,
        tlas: &crate::accel_structure::SceneTlas,
        light_direction: [f32; 3],
    ) {
        let shadow_view = self.shadow_view.as_ref().expect("call ensure_output first");

        // Upload light direction
        let light_data = RtShadowLightData {
            direction: light_direction,
            max_distance: 200.0,
        };
        queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light_data));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RT Shadow BG"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(gbuffer_position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tlas.tlas.as_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RT Shadow Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
    }
}

use wgpu::util::DeviceExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_shadow_shader_valid() {
        assert!(RT_SHADOW_SHADER.contains("@compute"));
        assert!(RT_SHADOW_SHADER.contains("acceleration_structure"));
        assert!(RT_SHADOW_SHADER.contains("rayQueryInitialize"));
        assert!(RT_SHADOW_SHADER.contains("rayQueryProceed"));
        assert!(RT_SHADOW_SHADER.contains("RAY_QUERY_INTERSECTION_NONE"));
    }

    #[test]
    fn test_rt_shadow_light_data_size() {
        assert_eq!(std::mem::size_of::<RtShadowLightData>(), 16);
    }
}
