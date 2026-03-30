//! GPU vertex formats and uniform types for deferred rendering.

use abrash_core::math::Mat4;
use bytemuck::{Pod, Zeroable};

// ---------------------------------------------------------------------------
// Vertex formats
// ---------------------------------------------------------------------------

/// Position + normal vertex for lit rendering (24 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LitVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl LitVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    #[must_use]
    pub(crate) const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Position + normal + UV vertex for textured lit rendering (32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TexturedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl TexturedVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    #[must_use]
    pub(crate) const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// ---------------------------------------------------------------------------
// Uniform types
// ---------------------------------------------------------------------------

/// Maximum number of lights supported per frame.
pub const MAX_LIGHTS: usize = 8;

/// Per-frame uniform data: camera + light metadata (bind group 0, binding 0).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniforms {
    pub view_proj: [f32; 16],
    pub camera_pos: [f32; 4],
    pub light_count: u32,
    pub(crate) _pad: [u32; 3],
}

/// GPU-side light data (48 bytes, bind group 0, binding 1).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLightData {
    pub position_or_direction: [f32; 3],
    pub light_type: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub(crate) _pad: [f32; 3],
}

/// Per-draw uniform data: model matrix + material properties (bind group 1, binding 0).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DrawUniforms {
    pub model: [f32; 16],
    pub color: [f32; 4],
    pub shininess: f32,
    pub specular_strength: f32,
    pub metallic: f32,
    pub roughness: f32,
}

impl DrawUniforms {
    #[must_use]
    pub fn new(model: &Mat4, argb: u32, shininess: f32, specular_strength: f32) -> Self {
        let flat: [f32; 16] = bytemuck::cast(model.m);
        let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
        let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
        let b = (argb & 0xFF) as f32 / 255.0;

        Self {
            model: flat,
            color: [r, g, b, a],
            shininess,
            specular_strength,
            metallic: 0.0,
            roughness: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lit_vertex_size_and_layout() {
        assert_eq!(std::mem::size_of::<LitVertex>(), 24);
        let layout = LitVertex::layout();
        assert_eq!(layout.array_stride, 24);
        assert_eq!(layout.attributes.len(), 2);
    }

    #[test]
    fn test_textured_vertex_size() {
        assert_eq!(std::mem::size_of::<TexturedVertex>(), 32);
        let layout = TexturedVertex::layout();
        assert_eq!(layout.array_stride, 32);
        assert_eq!(layout.attributes.len(), 3);
    }

    #[test]
    fn test_frame_uniforms_size() {
        let size = std::mem::size_of::<FrameUniforms>();
        assert_eq!(size, 96);
        assert_eq!(size % 16, 0);
    }

    #[test]
    fn test_gpu_light_data_size() {
        let size = std::mem::size_of::<GpuLightData>();
        assert_eq!(size, 48);
        assert_eq!(size % 16, 0);
    }

    #[test]
    fn test_draw_uniforms_size() {
        let size = std::mem::size_of::<DrawUniforms>();
        assert_eq!(size % 16, 0);
    }

    #[test]
    fn test_draw_uniforms_new() {
        let model = Mat4::identity();
        let u = DrawUniforms::new(&model, 0xFFFF_0000, 32.0, 0.5);
        assert!((u.color[0] - 1.0).abs() < f32::EPSILON);
        assert!(u.color[1].abs() < f32::EPSILON);
        assert!((u.shininess - 32.0).abs() < f32::EPSILON);
        assert!((u.specular_strength - 0.5).abs() < f32::EPSILON);
    }
}
