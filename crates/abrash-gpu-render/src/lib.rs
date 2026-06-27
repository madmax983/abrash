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
pub mod refraction;
pub mod renderer;
#[cfg(feature = "ray-tracing")]
pub mod rt_reflections;
pub mod primitives;

pub use demo_app::*;
pub use offscreen_bench::*;
pub use primitives::*;
pub mod demo_app;
pub mod offscreen_bench;
pub mod shader;
pub mod shadow;
#[cfg(feature = "windowed")]
pub mod surface;
pub mod svgf;
pub mod taa;
pub mod temporal;

use bytemuck::{Pod, Zeroable};

#[cfg(feature = "windowed")]
use std::time::Instant;
use wgpu::util::DeviceExt;
#[cfg(feature = "windowed")]
use winit::window::Window;

pub(crate) const SHADER_SRC: &str = r"
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
pub(crate) struct SceneUniform {
    yaw: f32,
    pitch: f32,
    aspect: f32,
    distance: f32,
}


