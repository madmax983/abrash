//! Final composition and debug visualization pass.
//!
//! Combines all render results (deferred lighting, RT reflections, skybox) into
//! the final HDR output. Provides a debug mode to visualize individual G-Buffer
//! channels and intermediate render targets.
//!
//! Maps to Pass 7 (Composition) and Pass 9 (Debug) from the paper.

use wgpu::util::DeviceExt;

/// Debug visualization modes for inspecting render pipeline stages.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default)]
pub enum DebugMode {
    /// Normal rendering (no debug overlay).
    #[default]
    None = 0,
    /// Visualize G-Buffer world positions (RGB = XYZ).
    GBufferPosition = 1,
    /// Visualize G-Buffer normals (RGB = XYZ mapped to \[0,1\]).
    GBufferNormal = 2,
    /// Visualize G-Buffer albedo (raw base color).
    GBufferAlbedo = 3,
    /// Visualize roughness (grayscale from normal.w).
    Roughness = 4,
    /// Visualize metallic (grayscale from albedo.a).
    Metallic = 5,
    /// Visualize shadow map / RT shadow factor.
    Shadows = 6,
    /// Visualize depth (linearized).
    Depth = 7,
}

/// WGSL shader for final composition with debug visualization.
///
/// Reads the composited HDR buffer and optionally overlays debug views
/// of individual G-Buffer channels or intermediate render targets.
const COMPOSITION_SHADER: &str = r"
struct CompositionParams {
    debug_mode: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var hdr_input: texture_2d<f32>;        // composited HDR
@group(0) @binding(1) var gbuf_position: texture_2d<f32>;
@group(0) @binding(2) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(3) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: CompositionParams;
@group(0) @binding(5) var output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(hdr_input);
    if gid.x >= size.x || gid.y >= size.y { return; }

    var color: vec3<f32>;

    switch params.debug_mode {
        // Normal rendering
        case 0u: {
            color = textureLoad(hdr_input, gid.xy, 0).rgb;
        }
        // G-Buffer position
        case 1u: {
            let pos = textureLoad(gbuf_position, gid.xy, 0).xyz;
            color = fract(pos * 0.1); // wrap to [0,1] for visibility
        }
        // G-Buffer normal
        case 2u: {
            let n = textureLoad(gbuf_normal, gid.xy, 0).xyz;
            color = n * 0.5 + 0.5; // map [-1,1] to [0,1]
        }
        // G-Buffer albedo
        case 3u: {
            color = textureLoad(gbuf_albedo, gid.xy, 0).rgb;
        }
        // Roughness
        case 4u: {
            let r = textureLoad(gbuf_normal, gid.xy, 0).w;
            color = vec3(r);
        }
        // Metallic
        case 5u: {
            let m = textureLoad(gbuf_albedo, gid.xy, 0).a;
            color = vec3(m);
        }
        // Shadows (placeholder — reads from HDR luminance as proxy)
        case 6u: {
            let lum = dot(textureLoad(hdr_input, gid.xy, 0).rgb, vec3(0.2126, 0.7152, 0.0722));
            color = vec3(lum);
        }
        // Depth
        case 7u: {
            let pos = textureLoad(gbuf_position, gid.xy, 0);
            if pos.w > 0.5 {
                let d = length(pos.xyz) / 100.0; // normalize to visible range
                color = vec3(1.0 - d);
            } else {
                color = vec3(0.0);
            }
        }
        default: {
            color = textureLoad(hdr_input, gid.xy, 0).rgb;
        }
    }

    textureStore(output, gid.xy, vec4(color, 1.0));
}
";

/// Composition parameters uniform.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CompositionParams {
    pub debug_mode: u32,
    pub(crate) _pad: [u32; 3],
}

/// Final composition pass with debug visualization support.
#[allow(dead_code)] // Fields used when composition is wired into renderer
pub struct CompositionPass {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    /// Current debug mode.
    pub debug_mode: DebugMode,
}

impl CompositionPass {
    /// Create the composition pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Composition Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITION_SHADER.into()),
        });

        let bind_group_layout = create_composition_bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Composition Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Composition Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Composition Params"),
            contents: bytemuck::bytes_of(&CompositionParams {
                debug_mode: 0,
                _pad: [0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            debug_mode: DebugMode::None,
        }
    }

    /// Set the debug visualization mode.
    pub const fn set_debug_mode(&mut self, mode: DebugMode) {
        self.debug_mode = mode;
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_shader_valid() {
        assert!(COMPOSITION_SHADER.contains("@compute"));
        assert!(COMPOSITION_SHADER.contains("debug_mode"));
        assert!(COMPOSITION_SHADER.contains("gbuf_position"));
        assert!(COMPOSITION_SHADER.contains("gbuf_normal"));
        assert!(COMPOSITION_SHADER.contains("gbuf_albedo"));
        // All 8 debug modes should be present
        for i in 0..8 {
            assert!(
                COMPOSITION_SHADER.contains(&format!("case {i}u")),
                "missing debug mode case {i}"
            );
        }
    }

    #[test]
    fn test_composition_params_size() {
        assert_eq!(std::mem::size_of::<CompositionParams>(), 16);
    }

    #[test]
    fn test_debug_mode_values() {
        assert_eq!(DebugMode::None as u32, 0);
        assert_eq!(DebugMode::GBufferPosition as u32, 1);
        assert_eq!(DebugMode::Depth as u32, 7);
    }
}

fn create_composition_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Composition Layout"),
        entries: &[
            // 0: HDR input
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
            // 1: G-Buffer position
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // 2: G-Buffer normal
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // 3: G-Buffer albedo
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // 4: params
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<CompositionParams>() as u64,
                    ),
                },
                count: None,
            },
            // 5: output
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba16Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    })
}
