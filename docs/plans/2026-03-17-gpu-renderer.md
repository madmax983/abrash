# Phase 5: GPU Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add GPU rendering to `abrash-gpu-render` with two paths: windowed display via wgpu swapchain, and headless debug capture with LLM-friendly diagnostic output.

**Architecture:** `GpuRenderer` consumes the existing `Frame` type from `abrash-render` (sharing `MeshHandle`, `MaterialHandle`, `FrameCamera`, `Light`). It manages its own GPU resource pools, uploads geometry via `GpuMeshBuffer`, and dispatches draw calls through a new MVP WGSL shader. Two output paths: `render_to_surface()` for windowed display (feature-gated behind `windowed`), and `capture()` for headless debug with pixel readback and structured diagnostics.

**Tech Stack:** wgpu 0.20.1, winit 0.29.15 (optional), bytemuck, pollster, abrash-core, abrash-render

**Key Design Decisions:**
- `GpuRenderer` does **NOT** implement the `Renderer` trait (incompatible target types — GPU renders to wgpu surfaces/textures, not `RenderTarget`)
- `Frame` is shared between CPU and GPU renderers — same submission API, different backends
- Mat4 is row-major, row-vector (`v' = v * M`). WGSL shader uses `vec4<f32>(pos, 1.0) * mvp` — no transpose needed
- `winit` is feature-gated behind `windowed` so headless capture has zero windowing dependency
- `GpuDebugCapture` produces compact text (LLM-parseable) rather than raw pixels or verbose JSON

**Module Layout:**
```
crates/abrash-gpu-render/src/
├── lib.rs            (existing demo code — untouched for backward compat)
├── device.rs         (GpuDevice: adapter/device/queue factory)
├── shader.rs         (MVP WGSL shader source + MvpPipeline)
├── mesh_buffer.rs    (GpuMeshBuffer: Mesh → GPU vertex/index buffers)
├── capture.rs        (GpuCaptureTarget, GpuDebugCapture, FrameStats)
├── surface.rs        (GpuSurface — feature-gated behind `windowed`)
└── renderer.rs       (GpuRenderer: core struct with both render paths)
```

---

### Task 1: ADR 007 + Cargo.toml + Module Scaffold

**Files:**
- Create: `docs/adr/007-gpu-renderer.md`
- Modify: `crates/abrash-gpu-render/Cargo.toml`
- Create: `crates/abrash-gpu-render/src/device.rs`
- Create: `crates/abrash-gpu-render/src/shader.rs`
- Create: `crates/abrash-gpu-render/src/mesh_buffer.rs`
- Create: `crates/abrash-gpu-render/src/capture.rs`
- Create: `crates/abrash-gpu-render/src/surface.rs`
- Create: `crates/abrash-gpu-render/src/renderer.rs`
- Modify: `crates/abrash-gpu-render/src/lib.rs`

**Step 1: Write ADR 007**

```markdown
# docs/adr/007-gpu-renderer.md

# ADR 007: GPU Renderer Design

## Status
Accepted

## Context

Phase 4 proved the engine API seam is correct: `embed-demo` depends only on `abrash-core`
+ `abrash-render` with zero platform dependencies. The `CpuRenderer` handles all rendering
via software rasterization through `TileRenderer`.

For real-time applications (games, editors), GPU rendering is needed. The existing
`abrash-gpu-render` crate has a standalone wgpu demo (`GpuMeshApp`) with its own
yaw/pitch/distance shader that doesn't integrate with the engine's `Frame`/`Mesh`/`Mat4`
types. We need a `GpuRenderer` that consumes the same `Frame` type as `CpuRenderer`,
enabling backend-agnostic scene submission.

## Decision

### GpuRenderer does NOT implement the Renderer trait

The `Renderer` trait couples `render_frame` to `RenderTarget` (CPU-side Framebuffer +
ZBuffer). GPU rendering targets either a wgpu swapchain surface (windowed) or a wgpu
texture (headless capture). Forcing these through `RenderTarget` would require a readback
every frame — defeating the purpose of GPU rendering.

Instead, `GpuRenderer` has its own API:
- `render_to_surface(frame, surface)` — windowed display, no readback
- `capture(frame, target) -> GpuDebugCapture` — headless with readback + diagnostics

Both consume `Frame` from `abrash-render`, sharing `MeshHandle`, `MaterialHandle`, etc.

### Two output paths

**Track A — Windowed (`render_to_surface`):**
- wgpu surface from winit window
- Swapchain present, no pixel readback
- Feature-gated behind `windowed` (winit dependency)

**Track B — Headless capture (`capture`):**
- Offscreen `Rgba8Unorm` texture + readback buffer
- 256-byte row alignment for wgpu `bytes_per_row` requirement
- `GpuDebugCapture` with pixel data, frame stats, and compact text dump
- No winit dependency — works in CI, test harnesses, LLM agents

### Row-major matrix convention

abrash-core's `Mat4` is `#[repr(C, align(16))] [[f32; 4]; 4]`, row-major, row-vector
(`v' = v * M`). When uploaded as bytes, WGSL interprets the data as column-major. The
WGSL shader uses `vec4<f32>(pos, 1.0) * mvp` (row-vector multiply) which produces the
correct result without transposing.

### Compact diagnostic format

`GpuDebugCapture` produces a compact text dump designed for LLM consumption:
```
FRAME 0 320x240 2 batches 24 tris 1.2ms
  B0 12t color=0xFFFF4444 visible=236px
  B1 12t color=0xFF4444FF visible=189px
COVERAGE 5.5% (4248/76800)
```

Dense, self-documenting, no nested structure. An LLM can parse this without JSON overhead.

## Consequences

- Consumers use `Frame` for both CPU and GPU paths — identical scene submission
- Handles from one renderer cannot be used with another (different resource pools)
- Headless capture works without a window — enables GPU debugging in CI/agentic workflows
- Existing `GpuMeshApp`/`GpuOffscreenBench` demo code is preserved for backward compat
```

**Step 2: Update Cargo.toml**

```toml
# crates/abrash-gpu-render/Cargo.toml
[package]
name = "abrash-gpu-render"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[features]
default = ["windowed"]
windowed = ["dep:winit"]

[dependencies]
abrash-core = { path = "../abrash-core" }
abrash-render = { path = "../abrash-render", default-features = false }
wgpu = "0.20.1"
winit = { version = "0.29.15", optional = true }
pollster = "0.3.0"
bytemuck = { version = "1.24.0", features = ["derive"] }

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cast_possible_truncation = "allow"
cast_possible_wrap = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
```

**Step 3: Create module scaffold files**

Each new module file starts with a doc comment and placeholder:

```rust
// device.rs
//! GPU device, adapter, and queue factory.

// shader.rs
//! MVP WGSL shader and render pipeline construction.

// mesh_buffer.rs
//! Converts `abrash_core::mesh::Mesh` to GPU vertex/index buffers.

// capture.rs
//! Headless GPU capture target and diagnostic output.

// surface.rs
//! Windowed display surface (feature-gated behind `windowed`).

// renderer.rs
//! `GpuRenderer` — the GPU rendering backend.
```

**Step 4: Update lib.rs to declare new modules**

Add to the top of `crates/abrash-gpu-render/src/lib.rs`, before existing code:

```rust
pub mod device;
pub mod shader;
pub mod mesh_buffer;
pub mod capture;
#[cfg(feature = "windowed")]
pub mod surface;
pub mod renderer;
```

Keep all existing code intact. The existing `GpuMeshApp`, `GpuOffscreenBench`, etc.
remain in `lib.rs` unchanged for backward compatibility.

**Step 5: Verify it compiles**

Run: `cargo check -p abrash-gpu-render --features windowed`
Expected: compiles with no errors (modules are empty/placeholder)

Run: `cargo check -p abrash-gpu-render --no-default-features`
Expected: compiles without winit

**Step 6: Commit**

```bash
git add docs/adr/007-gpu-renderer.md crates/abrash-gpu-render/
git commit -m "feat(gpu-render): ADR 007 + module scaffold + abrash-core/render deps"
```

---

### Task 2: GpuDevice Factory

**Files:**
- Modify: `crates/abrash-gpu-render/src/device.rs`
- Test: inline `#[cfg(test)]` module

**Context:** Both `GpuMeshApp` and `GpuOffscreenBench` duplicate ~30 lines of adapter/device/queue setup. `GpuDevice` consolidates this into a reusable factory. The factory also produces a headless device (no surface) for the capture path.

**Step 1: Write the failing test**

```rust
// device.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_device_config_default() {
        let config = GpuDeviceConfig::default();
        assert_eq!(config.power_preference, wgpu::PowerPreference::HighPerformance);
        assert!(!config.force_fallback);
    }

    #[test]
    fn test_gpu_device_config_headless() {
        let config = GpuDeviceConfig::headless();
        assert_eq!(config.power_preference, wgpu::PowerPreference::HighPerformance);
        assert!(!config.force_fallback);
        assert!(config.compatible_surface.is_none()); // headless = no surface
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-gpu-render test_gpu_device_config`
Expected: FAIL — `GpuDeviceConfig` not defined

**Step 3: Write minimal implementation**

```rust
// device.rs
//! GPU device, adapter, and queue factory.

/// Configuration for GPU device creation.
#[derive(Debug, Clone)]
pub struct GpuDeviceConfig {
    /// GPU power preference.
    pub power_preference: wgpu::PowerPreference,
    /// Force software fallback adapter (for CI/testing).
    pub force_fallback: bool,
    /// Compatible surface (None for headless).
    pub compatible_surface: Option<()>, // placeholder — real surface passed at creation time
}

impl Default for GpuDeviceConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback: false,
            compatible_surface: None,
        }
    }
}

impl GpuDeviceConfig {
    /// Config for headless rendering (no window surface).
    #[must_use]
    pub fn headless() -> Self {
        Self::default()
    }
}

/// Owns the wgpu device, queue, and adapter — shared by all render paths.
pub struct GpuDevice {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::Adapter,
}

impl GpuDevice {
    /// Create a headless GPU device (no surface required).
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable GPU adapter is found.
    pub fn new_headless(config: &GpuDeviceConfig) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: config.power_preference,
            compatible_surface: None,
            force_fallback_adapter: config.force_fallback,
        }))
        .ok_or_else(|| "No suitable GPU adapter found".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Abrash GpuDevice"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))
        .map_err(|e| format!("Failed to create GPU device: {e}"))?;

        Ok(Self {
            device,
            queue,
            adapter,
        })
    }

    /// The underlying wgpu device.
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The underlying wgpu queue.
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-gpu-render test_gpu_device_config`
Expected: PASS (2/2 config tests)

**Step 5: Commit**

```bash
git add crates/abrash-gpu-render/src/device.rs
git commit -m "feat(gpu-render): GpuDevice factory for adapter/device/queue creation"
```

---

### Task 3: MVP WGSL Shader + Uniform Types

**Files:**
- Modify: `crates/abrash-gpu-render/src/shader.rs`
- Test: inline `#[cfg(test)]` module

**Context:** The existing shader uses `yaw/pitch/distance` uniforms with manual trig in the vertex shader. The new MVP shader takes a `mat4x4<f32>` MVP matrix and `vec4<f32>` color uniform. abrash-core's `Mat4` is row-major — WGSL interprets the uploaded bytes as column-major, so the shader uses `vec4<f32>(pos, 1.0) * mvp` (row-vector multiply) which produces the correct result without transposing.

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvp_uniform_size() {
        // 4×4 f32 matrix = 64 bytes, + 4×f32 color = 16 bytes = 80 bytes
        assert_eq!(std::mem::size_of::<MvpUniform>(), 80);
    }

    #[test]
    fn test_mvp_uniform_from_mat4() {
        use abrash_core::math::Mat4;
        let identity = Mat4::identity();
        let uniform = MvpUniform::new(&identity, 0xFFFF0000);
        // Row 0 of identity: [1, 0, 0, 0]
        assert_eq!(uniform.mvp[0], 1.0);
        assert_eq!(uniform.mvp[1], 0.0);
        // Color: 0xFFFF0000 → RGBA [1.0, 0.0, 0.0, 1.0]
        assert_eq!(uniform.color[0], 1.0); // R
        assert_eq!(uniform.color[1], 0.0); // G
        assert_eq!(uniform.color[2], 0.0); // B
        assert!((uniform.color[3] - 1.0).abs() < 0.01); // A
    }

    #[test]
    fn test_shader_source_is_valid_wgsl() {
        // Smoke test: shader source contains expected entry points
        assert!(MVP_SHADER_SRC.contains("fn vs_main"));
        assert!(MVP_SHADER_SRC.contains("fn fs_main"));
        assert!(MVP_SHADER_SRC.contains("mvp"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-gpu-render test_mvp_uniform`
Expected: FAIL — `MvpUniform` and `MVP_SHADER_SRC` not defined

**Step 3: Write implementation**

```rust
// shader.rs
//! MVP WGSL shader and uniform types.
//!
//! The shader accepts a combined model-view-projection matrix and flat color.
//! Vertex positions are transformed on the GPU — this is the natural GPU path,
//! unlike the CPU `DrawList` which pre-transforms to clip-space.
//!
//! # Matrix Convention
//!
//! abrash-core's `Mat4` is row-major, row-vector (`v' = v * M`).
//! When uploaded as raw bytes, WGSL interprets them as column-major.
//! The shader uses `vec4<f32>(pos, 1.0) * mvp` (row-vector multiply)
//! which produces the correct clip-space position without transposing.

use abrash_core::math::Mat4;
use bytemuck::{Pod, Zeroable};

/// WGSL shader source for MVP-based rendering with flat color.
pub const MVP_SHADER_SRC: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    // Row-vector multiply: pos * MVP (abrash-core is row-major,
    // uploaded bytes are interpreted as column-major by WGSL,
    // so v * M_wgsl gives the correct result).
    var out: VsOut;
    out.clip_position = vec4<f32>(input.position, 1.0) * uniforms.mvp;
    out.color = uniforms.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/// GPU uniform buffer data: MVP matrix + flat color.
///
/// Uploaded once per draw call. Layout must match the WGSL `Uniforms` struct.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MvpUniform {
    /// Combined model-view-projection matrix (64 bytes, row-major as stored in Mat4).
    pub mvp: [f32; 16],
    /// Surface color as linear RGBA floats.
    pub color: [f32; 4],
}

impl MvpUniform {
    /// Create a uniform from a `Mat4` and a 0xAARRGGBB color.
    #[must_use]
    pub fn new(mvp: &Mat4, argb: u32) -> Self {
        // Flatten row-major Mat4 to [f32; 16] — direct memory copy
        let mut mvp_flat = [0.0f32; 16];
        for (row_idx, row) in mvp.m.iter().enumerate() {
            for (col_idx, &val) in row.iter().enumerate() {
                mvp_flat[row_idx * 4 + col_idx] = val;
            }
        }

        let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
        let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
        let b = (argb & 0xFF) as f32 / 255.0;

        Self {
            mvp: mvp_flat,
            color: [r, g, b, a],
        }
    }
}

/// Vertex layout for the MVP shader: position only (3 floats).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MvpVertex {
    pub position: [f32; 3],
}

impl MvpVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 1] =
        wgpu::vertex_attr_array![0 => Float32x3];

    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-gpu-render test_mvp_uniform && cargo test -p abrash-gpu-render test_shader_source`
Expected: PASS (3/3 tests)

**Step 5: Commit**

```bash
git add crates/abrash-gpu-render/src/shader.rs
git commit -m "feat(gpu-render): MVP WGSL shader + MvpUniform type with row-vector convention"
```

---

### Task 4: GpuMeshBuffer

**Files:**
- Modify: `crates/abrash-gpu-render/src/mesh_buffer.rs`
- Test: inline `#[cfg(test)]` module

**Context:** Converts `abrash_core::mesh::Mesh` (Vec3 vertices, `[usize; 3]` indices) to GPU buffers (`MvpVertex` position-only vertices, `u32` indices). The index type is widened from `usize` to `u32` — meshes with >4B vertices are rejected.

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::mesh::Mesh;

    #[test]
    fn test_prepare_vertices_from_cube() {
        let mesh = Mesh::cube(1.0);
        let (verts, indices) = prepare_mesh_data(&mesh).unwrap();
        assert_eq!(verts.len(), mesh.vertices.len());
        assert_eq!(indices.len(), mesh.indices.len() * 3); // flattened
        // First vertex of unit cube is at (-0.5, -0.5, 0.5)
        assert_eq!(verts[0].position[0], -0.5);
    }

    #[test]
    fn test_prepare_empty_mesh() {
        let mesh = Mesh::new();
        let result = prepare_mesh_data(&mesh);
        assert!(result.is_err());
    }

    #[test]
    fn test_indices_are_u32() {
        let mesh = Mesh::cube(1.0);
        let (_, indices) = prepare_mesh_data(&mesh).unwrap();
        // All indices should be small for a cube
        for &idx in &indices {
            assert!(idx < 8);
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-gpu-render test_prepare`
Expected: FAIL — `prepare_mesh_data` not defined

**Step 3: Write implementation**

```rust
// mesh_buffer.rs
//! Converts `abrash_core::mesh::Mesh` to GPU vertex/index buffers.

use crate::shader::MvpVertex;
use abrash_core::mesh::Mesh;
use wgpu::util::DeviceExt;

/// Converts a `Mesh` into GPU-ready vertex and index arrays.
///
/// Returns `(vertices, flat_indices)` where indices are `u32` (widened from `usize`).
///
/// # Errors
///
/// Returns an error if the mesh is empty or has out-of-bounds indices.
pub fn prepare_mesh_data(mesh: &Mesh) -> Result<(Vec<MvpVertex>, Vec<u32>), String> {
    if mesh.vertices.is_empty() {
        return Err("mesh has no vertices".to_string());
    }
    if mesh.indices.is_empty() {
        return Err("mesh has no indices".to_string());
    }

    let vertices: Vec<MvpVertex> = mesh
        .vertices
        .iter()
        .map(|v| MvpVertex {
            position: [v.x, v.y, v.z],
        })
        .collect();

    let vertex_count = vertices.len();
    let mut indices = Vec::with_capacity(mesh.indices.len() * 3);
    for tri in &mesh.indices {
        for &idx in tri {
            if idx >= vertex_count {
                return Err(format!(
                    "index {idx} out of bounds for {vertex_count} vertices"
                ));
            }
            indices.push(idx as u32);
        }
    }

    Ok((vertices, indices))
}

/// A mesh uploaded to the GPU, ready for draw calls.
pub struct GpuMeshBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) triangle_count: u32,
}

impl GpuMeshBuffer {
    /// Upload a `Mesh` to the GPU.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn from_mesh(device: &wgpu::Device, mesh: &Mesh) -> Result<Self, String> {
        let (vertices, indices) = prepare_mesh_data(mesh)?;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GpuMeshBuffer Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_count = indices.len() as u32;
        let triangle_count = index_count / 3;

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GpuMeshBuffer Indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_count,
            triangle_count,
        })
    }

    /// Number of indices (draw_indexed count).
    #[must_use]
    pub const fn index_count(&self) -> u32 {
        self.index_count
    }

    /// Number of triangles.
    #[must_use]
    pub const fn triangle_count(&self) -> u32 {
        self.triangle_count
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-gpu-render test_prepare`
Expected: PASS (3/3 tests)

**Step 5: Commit**

```bash
git add crates/abrash-gpu-render/src/mesh_buffer.rs
git commit -m "feat(gpu-render): GpuMeshBuffer — Mesh to GPU vertex/index buffer conversion"
```

---

### Task 5: MvpPipeline

**Files:**
- Modify: `crates/abrash-gpu-render/src/shader.rs` (append pipeline builder)

**Context:** Wraps wgpu shader module + pipeline + bind group layout + uniform buffer into a reusable pipeline object. Both windowed and headless paths share this pipeline — only the color target format differs (`Rgba8Unorm` for capture, surface format for windowed).

**Step 1: Write the failing test**

```rust
// Add to shader.rs tests
#[test]
fn test_mvp_vertex_layout_stride() {
    let layout = MvpVertex::layout();
    assert_eq!(layout.array_stride, 12); // 3 * f32 = 12 bytes
    assert_eq!(layout.attributes.len(), 1);
}
```

**Step 2: Run test to verify it fails/passes**

Run: `cargo test -p abrash-gpu-render test_mvp_vertex_layout_stride`
Expected: Should PASS (already implemented in Task 3). This validates the vertex layout.

**Step 3: Write MvpPipeline implementation**

Append to `shader.rs`:

```rust
/// Pre-built wgpu render pipeline for MVP-based rendering.
///
/// Shared by both windowed and headless capture paths.
pub struct MvpPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl MvpPipeline {
    /// Create a render pipeline for the given color target format.
    ///
    /// Use `wgpu::TextureFormat::Rgba8Unorm` for headless capture,
    /// or the surface's preferred format for windowed rendering.
    #[must_use]
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Abrash MVP Shader"),
            source: wgpu::ShaderSource::Wgsl(MVP_SHADER_SRC.into()),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MVP Uniform Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MVP Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Abrash MVP Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[MvpVertex::layout()],
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
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
        }
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check -p abrash-gpu-render`
Expected: compiles (pipeline creation is device-dependent, no pure unit test needed)

**Step 5: Commit**

```bash
git add crates/abrash-gpu-render/src/shader.rs
git commit -m "feat(gpu-render): MvpPipeline — reusable wgpu render pipeline for MVP shader"
```

---

### Task 6: GpuCaptureTarget

**Files:**
- Modify: `crates/abrash-gpu-render/src/capture.rs`
- Test: inline `#[cfg(test)]` module

**Context:** Offscreen render target with a `Rgba8Unorm` color texture, `Depth24Plus` depth texture, and a readback buffer. The readback buffer's `bytes_per_row` must be aligned to 256 bytes (wgpu requirement). The target owns the textures and readback buffer; `GpuRenderer::capture()` renders into it and reads back pixels.

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_bytes_per_row() {
        // 320 pixels * 4 bytes = 1280, already aligned to 256
        assert_eq!(aligned_bytes_per_row(320), 1280);
        // 100 pixels * 4 bytes = 400 → next multiple of 256 = 512
        assert_eq!(aligned_bytes_per_row(100), 512);
        // 1 pixel * 4 bytes = 4 → next multiple of 256 = 256
        assert_eq!(aligned_bytes_per_row(1), 256);
        // 1920 pixels * 4 bytes = 7680, already aligned
        assert_eq!(aligned_bytes_per_row(1920), 7680);
    }

    #[test]
    fn test_capture_target_dimensions() {
        // This test validates the config, not the GPU resources
        let config = CaptureConfig::new(320, 240);
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 240);
        assert_eq!(config.padded_bytes_per_row, aligned_bytes_per_row(320));
        assert_eq!(config.unpadded_bytes_per_row, 320 * 4);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-gpu-render test_aligned`
Expected: FAIL — function not defined

**Step 3: Write implementation**

```rust
// capture.rs
//! Headless GPU capture target and diagnostic output.
//!
//! `GpuCaptureTarget` is an offscreen render target backed by wgpu textures.
//! After rendering, pixels are read back to CPU memory and analyzed to produce
//! a `GpuDebugCapture` with frame statistics and a compact text dump suitable
//! for LLM consumption.

use std::time::Duration;

/// Compute `bytes_per_row` aligned to wgpu's 256-byte requirement.
///
/// wgpu requires `bytes_per_row` in texture copy operations to be a multiple of
/// `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (256).
#[must_use]
pub fn aligned_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4; // 4 bytes per RGBA pixel
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    (unpadded + align - 1) / align * align
}

/// Configuration for a capture target (pure data, no GPU resources).
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub unpadded_bytes_per_row: u32,
}

impl CaptureConfig {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            padded_bytes_per_row: aligned_bytes_per_row(width),
            unpadded_bytes_per_row: width * 4,
        }
    }
}

/// Offscreen GPU render target with readback capability.
///
/// Owns color texture (`Rgba8Unorm`), depth texture, and a staging buffer
/// for reading pixels back to CPU memory.
pub struct GpuCaptureTarget {
    pub(crate) config: CaptureConfig,
    pub(crate) color_texture: wgpu::Texture,
    pub(crate) color_view: wgpu::TextureView,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) readback_buffer: wgpu::Buffer,
}

impl GpuCaptureTarget {
    /// Create a new capture target.
    ///
    /// Uses `Rgba8Unorm` (linear, not sRGB) so readback pixel values match
    /// what the shader wrote without gamma correction artifacts.
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = CaptureConfig::new(width, height);

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Capture Color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Capture Depth"),
            size: wgpu::Extent3d {
                width,
                height,
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

        let readback_size = config.padded_bytes_per_row as u64 * u64::from(height);
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Capture Readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            config,
            color_texture,
            color_view,
            depth_texture,
            depth_view,
            readback_buffer,
        }
    }

    /// Width of the capture target.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Height of the capture target.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.config.height
    }
}

/// Per-batch statistics in a captured frame.
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Batch index (submission order).
    pub index: usize,
    /// Triangle count for this batch.
    pub triangle_count: u32,
    /// Flat color (0xAARRGGBB).
    pub color: u32,
}

/// Frame-level statistics from a GPU capture.
#[derive(Debug, Clone)]
pub struct FrameStats {
    /// Resolution width.
    pub width: u32,
    /// Resolution height.
    pub height: u32,
    /// Total batches submitted.
    pub batch_count: usize,
    /// Total triangles across all batches.
    pub total_triangles: u32,
    /// GPU render + readback time.
    pub render_time: Duration,
    /// Per-batch statistics.
    pub batches: Vec<BatchStats>,
}

/// Result of a headless GPU capture: pixels + diagnostics.
#[derive(Debug)]
pub struct GpuDebugCapture {
    /// Frame statistics.
    pub stats: FrameStats,
    /// Raw RGBA pixels (row-major, no padding). Length = width * height * 4.
    pub pixels_rgba: Vec<u8>,
    /// Number of non-background pixels.
    pub visible_pixel_count: usize,
}

impl GpuDebugCapture {
    /// Total pixel count.
    #[must_use]
    pub fn total_pixels(&self) -> usize {
        self.stats.width as usize * self.stats.height as usize
    }

    /// Coverage percentage (visible / total).
    #[must_use]
    pub fn coverage_percent(&self) -> f32 {
        if self.total_pixels() == 0 {
            return 0.0;
        }
        self.visible_pixel_count as f32 / self.total_pixels() as f32 * 100.0
    }

    /// Compact text dump for LLM/developer consumption.
    ///
    /// Format:
    /// ```text
    /// FRAME 320x240 2 batches 24 tris 1.2ms
    ///   B0 12t color=0xFFFF4444
    ///   B1 12t color=0xFF4444FF
    /// COVERAGE 5.5% (4248/76800)
    /// ```
    #[must_use]
    pub fn to_compact_text(&self) -> String {
        let ms = self.stats.render_time.as_secs_f64() * 1000.0;
        let mut out = format!(
            "FRAME {}x{} {} batches {} tris {ms:.1}ms\n",
            self.stats.width,
            self.stats.height,
            self.stats.batch_count,
            self.stats.total_triangles,
        );

        for batch in &self.stats.batches {
            out.push_str(&format!(
                "  B{} {}t color=0x{:08X}\n",
                batch.index, batch.triangle_count, batch.color,
            ));
        }

        out.push_str(&format!(
            "COVERAGE {:.1}% ({}/{})\n",
            self.coverage_percent(),
            self.visible_pixel_count,
            self.total_pixels(),
        ));

        out
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-gpu-render test_aligned && cargo test -p abrash-gpu-render test_capture_target`
Expected: PASS (4/4 tests)

**Step 5: Write additional text formatter tests**

```rust
#[test]
fn test_compact_text_format() {
    let capture = GpuDebugCapture {
        stats: FrameStats {
            width: 320,
            height: 240,
            batch_count: 2,
            total_triangles: 24,
            render_time: Duration::from_micros(1200),
            batches: vec![
                BatchStats { index: 0, triangle_count: 12, color: 0xFFFF4444 },
                BatchStats { index: 1, triangle_count: 12, color: 0xFF4444FF },
            ],
        },
        pixels_rgba: vec![0; 320 * 240 * 4],
        visible_pixel_count: 4248,
    };
    let text = capture.to_compact_text();
    assert!(text.contains("FRAME 320x240 2 batches 24 tris"));
    assert!(text.contains("B0 12t color=0xFFFF4444"));
    assert!(text.contains("B1 12t color=0xFF4444FF"));
    assert!(text.contains("COVERAGE 5.5%"));
    assert!(text.contains("(4248/76800)"));
}

#[test]
fn test_coverage_percent() {
    let capture = GpuDebugCapture {
        stats: FrameStats {
            width: 100,
            height: 100,
            batch_count: 0,
            total_triangles: 0,
            render_time: Duration::ZERO,
            batches: vec![],
        },
        pixels_rgba: vec![0; 100 * 100 * 4],
        visible_pixel_count: 5000,
    };
    assert!((capture.coverage_percent() - 50.0).abs() < 0.01);
}
```

**Step 6: Run all capture tests**

Run: `cargo test -p abrash-gpu-render capture`
Expected: PASS (6/6 tests)

**Step 7: Commit**

```bash
git add crates/abrash-gpu-render/src/capture.rs
git commit -m "feat(gpu-render): GpuCaptureTarget + GpuDebugCapture with compact text dump"
```

---

### Task 7: GpuRenderer Core + Resource Pools

**Files:**
- Modify: `crates/abrash-gpu-render/src/renderer.rs`
- Test: inline `#[cfg(test)]` module

**Context:** `GpuRenderer` owns a `GpuDevice`, `MvpPipeline`, uniform buffer, and resource pools for meshes and materials. It consumes `Frame` from `abrash-render` and resolves handles against its own GPU-backed pools. Mesh handles from this renderer cannot be used with `CpuRenderer` (and vice versa).

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argb_to_clear_color() {
        let color = argb_to_wgpu_color(0xFF804020);
        // 0xFF = alpha 1.0, 0x80 ≈ 0.502, 0x40 ≈ 0.251, 0x20 ≈ 0.125
        assert!((color.r - 0.502).abs() < 0.01);
        assert!((color.g - 0.251).abs() < 0.01);
        assert!((color.b - 0.125).abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_argb_black() {
        let color = argb_to_wgpu_color(0xFF000000);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert!((color.a - 1.0).abs() < 0.01);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-gpu-render test_argb`
Expected: FAIL — `argb_to_wgpu_color` not defined

**Step 3: Write implementation**

```rust
// renderer.rs
//! `GpuRenderer` — the GPU rendering backend.
//!
//! Consumes `Frame` from `abrash-render` and renders via wgpu.
//! Manages its own GPU resource pools for meshes and materials.

use crate::capture::{
    BatchStats, CaptureConfig, FrameStats, GpuCaptureTarget, GpuDebugCapture,
};
use crate::device::{GpuDevice, GpuDeviceConfig};
use crate::mesh_buffer::GpuMeshBuffer;
use crate::shader::{MvpPipeline, MvpUniform};
use abrash_core::math::Mat4;
use abrash_core::mesh::Mesh;
use abrash_render::render_api::frame::Frame;
use abrash_render::render_api::handles::{Handle, MaterialHandle, MeshHandle};
use abrash_render::render_api::material::{Material, ShadingMode};
use std::time::Instant;
use wgpu::util::DeviceExt;

/// Convert 0xAARRGGBB to wgpu clear color.
#[must_use]
pub fn argb_to_wgpu_color(argb: u32) -> wgpu::Color {
    let a = ((argb >> 24) & 0xFF) as f64 / 255.0;
    let r = ((argb >> 16) & 0xFF) as f64 / 255.0;
    let g = ((argb >> 8) & 0xFF) as f64 / 255.0;
    let b = (argb & 0xFF) as f64 / 255.0;
    wgpu::Color { r, g, b, a }
}

/// Marker type for GPU mesh resources (distinguishes from CPU `MeshResource`).
struct GpuMeshResource;
/// Marker type for GPU material resources.
struct GpuMaterialResource;

/// A material stored in the GPU renderer's pool.
struct GpuMaterial {
    /// The flat color extracted from the material (0xAARRGGBB).
    /// Phase 5 only supports flat shading; future phases add texture handles.
    color: u32,
}

/// GPU rendering backend.
///
/// Renders `Frame` objects via wgpu. Does NOT implement the `Renderer` trait
/// (incompatible target types — GPU renders to surfaces/textures, not `RenderTarget`).
///
/// # Lifecycle
///
/// 1. `GpuRenderer::new()` — create the renderer
/// 2. `create_mesh(&mesh)` — upload geometry, receive a `MeshHandle`
/// 3. `create_material(material)` — register material, receive a `MaterialHandle`
/// 4. Build a `Frame`, call `capture()` or `render_to_surface()`
/// 5. Read `GpuDebugCapture` diagnostics (capture path) or enjoy the window (surface path)
pub struct GpuRenderer {
    gpu: GpuDevice,
    pipeline: MvpPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    meshes: Vec<Option<GpuMeshBuffer>>,
    materials: Vec<Option<GpuMaterial>>,
}

impl GpuRenderer {
    /// Create a headless GPU renderer.
    ///
    /// Uses `Rgba8Unorm` format for the pipeline (linear color, no sRGB).
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable GPU adapter is found.
    pub fn new_headless() -> Result<Self, String> {
        let config = GpuDeviceConfig::headless();
        let gpu = GpuDevice::new_headless(&config)?;

        let pipeline = MvpPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);

        // Create uniform buffer large enough for one MvpUniform.
        let uniform_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GpuRenderer Uniform"),
                contents: bytemuck::bytes_of(&MvpUniform::new(&Mat4::identity(), 0xFFFFFFFF)),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GpuRenderer Uniform Bind Group"),
            layout: &pipeline.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            gpu,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            meshes: Vec::new(),
            materials: Vec::new(),
        })
    }

    /// Upload a mesh and return a handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, String> {
        let gpu_mesh = GpuMeshBuffer::from_mesh(&self.gpu.device, mesh)?;
        let index = self.meshes.len() as u32;
        self.meshes.push(Some(gpu_mesh));
        Ok(Handle::new(index, 0))
    }

    /// Register a material and return a handle.
    ///
    /// Phase 5 extracts the flat color from the material. Future phases will
    /// support textured and lit materials on the GPU.
    pub fn create_material(&mut self, material: Material) -> MaterialHandle {
        let color = match material.shading {
            ShadingMode::Flat { color } => color,
            _ => material.color, // fallback to base color for non-flat modes
        };
        let index = self.materials.len() as u32;
        self.materials.push(Some(GpuMaterial { color }));
        Handle::new(index, 0)
    }

    /// Destroy a mesh (free GPU buffers).
    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        if let Some(slot) = self.meshes.get_mut(handle.index as usize) {
            *slot = None;
        }
    }

    /// Destroy a material.
    pub fn destroy_material(&mut self, handle: MaterialHandle) {
        if let Some(slot) = self.materials.get_mut(handle.index as usize) {
            *slot = None;
        }
    }

    /// Render a frame to a capture target and read back pixels + diagnostics.
    ///
    /// This is the headless debug path (Track B). The frame is rendered to an
    /// offscreen texture, pixels are read back to CPU memory, and a
    /// `GpuDebugCapture` is produced with statistics and a compact text dump.
    ///
    /// # Errors
    ///
    /// Returns an error if a mesh or material handle in the frame is invalid.
    pub fn capture(
        &mut self,
        frame: &Frame,
        target: &mut GpuCaptureTarget,
    ) -> Result<GpuDebugCapture, String> {
        let start = Instant::now();
        let view_proj = frame.camera.view * frame.camera.projection;

        // Determine clear color.
        let clear_color = frame
            .clear_color
            .map_or(wgpu::Color::BLACK, argb_to_wgpu_color);
        let bg_argb = frame.clear_color.unwrap_or(0xFF000000);

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GpuRenderer Capture Encoder"),
            });

        // Collect batch stats.
        let mut batch_stats = Vec::new();
        let mut total_triangles = 0u32;

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GpuRenderer Capture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            for (cmd_idx, cmd) in frame.commands.iter().enumerate() {
                // Resolve mesh handle.
                let gpu_mesh = self
                    .meshes
                    .get(cmd.mesh.index as usize)
                    .and_then(|slot| slot.as_ref())
                    .ok_or_else(|| format!("stale mesh handle at command {cmd_idx}"))?;

                // Resolve material handle.
                let gpu_mat = self
                    .materials
                    .get(cmd.material.index as usize)
                    .and_then(|slot| slot.as_ref())
                    .ok_or_else(|| format!("stale material handle at command {cmd_idx}"))?;

                // Compute MVP = model * view * projection
                let mvp = cmd.transform * view_proj;
                let uniform = MvpUniform::new(&mvp, gpu_mat.color);
                self.gpu.queue.write_buffer(
                    &self.uniform_buffer,
                    0,
                    bytemuck::bytes_of(&uniform),
                );

                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(
                    gpu_mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);

                batch_stats.push(BatchStats {
                    index: cmd_idx,
                    triangle_count: gpu_mesh.triangle_count,
                    color: gpu_mat.color,
                });
                total_triangles += gpu_mesh.triangle_count;
            }
        }

        // Copy color texture to readback buffer.
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &target.readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(target.config.padded_bytes_per_row),
                    rows_per_image: Some(target.config.height),
                },
            },
            wgpu::Extent3d {
                width: target.config.width,
                height: target.config.height,
                depth_or_array_layers: 1,
            },
        );

        self.gpu.queue.submit(Some(encoder.finish()));

        // Read back pixels.
        let buffer_slice = target.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).expect("channel send");
        });
        self.gpu.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| format!("readback channel error: {e}"))?
            .map_err(|e| format!("readback map error: {e}"))?;

        let mapped = buffer_slice.get_mapped_range();

        // Strip row padding to produce a contiguous RGBA pixel array.
        let w = target.config.width as usize;
        let h = target.config.height as usize;
        let padded_stride = target.config.padded_bytes_per_row as usize;
        let unpadded_stride = target.config.unpadded_bytes_per_row as usize;

        let mut pixels_rgba = Vec::with_capacity(w * h * 4);
        for row in 0..h {
            let row_start = row * padded_stride;
            pixels_rgba.extend_from_slice(&mapped[row_start..row_start + unpadded_stride]);
        }
        drop(mapped);
        target.readback_buffer.unmap();

        // Count visible (non-background) pixels.
        // Background color in RGBA from ARGB:
        let bg_r = ((bg_argb >> 16) & 0xFF) as u8;
        let bg_g = ((bg_argb >> 8) & 0xFF) as u8;
        let bg_b = (bg_argb & 0xFF) as u8;
        let bg_a = ((bg_argb >> 24) & 0xFF) as u8;

        let mut visible = 0usize;
        for pixel in pixels_rgba.chunks_exact(4) {
            if pixel[0] != bg_r || pixel[1] != bg_g || pixel[2] != bg_b || pixel[3] != bg_a {
                visible += 1;
            }
        }

        let render_time = start.elapsed();

        Ok(GpuDebugCapture {
            stats: FrameStats {
                width: target.config.width,
                height: target.config.height,
                batch_count: batch_stats.len(),
                total_triangles,
                render_time,
                batches: batch_stats,
            },
            pixels_rgba,
            visible_pixel_count: visible,
        })
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-gpu-render test_argb`
Expected: PASS (2/2 tests)

**Step 5: Verify compilation**

Run: `cargo check -p abrash-gpu-render --no-default-features`
Expected: compiles (GPU tests require a real adapter)

**Step 6: Commit**

```bash
git add crates/abrash-gpu-render/src/renderer.rs
git commit -m "feat(gpu-render): GpuRenderer with headless capture() path"
```

---

### Task 8: Capture Integration Test

**Files:**
- Create: `crates/abrash-gpu-render/tests/capture_test.rs`

**Context:** End-to-end test: create a `GpuRenderer`, upload a cube, render one frame via `capture()`, verify the `GpuDebugCapture` contains visible pixels and correct stats. This test requires a GPU adapter — it will be skipped in environments without one.

**Step 1: Write the integration test**

```rust
// crates/abrash-gpu-render/tests/capture_test.rs
//! Integration test: GPU headless capture of a cube.
//!
//! Requires a GPU adapter. Skipped (not failed) if no adapter is available.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::capture::GpuCaptureTarget;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;

fn try_create_renderer() -> Option<GpuRenderer> {
    GpuRenderer::new_headless().ok()
}

#[test]
fn test_capture_cube_has_visible_pixels() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).expect("create_mesh");
    let mat_h = renderer.create_material(Material::flat(0xFFFF4444));

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 320.0 / 240.0, 0.1, 100.0),
    );

    let mut frame = Frame::new(camera);
    frame.draw(mesh_h, mat_h, Mat4::identity());

    let mut target = GpuCaptureTarget::new(renderer.gpu.device(), 320, 240);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    // Verify stats.
    assert_eq!(capture.stats.width, 320);
    assert_eq!(capture.stats.height, 240);
    assert_eq!(capture.stats.batch_count, 1);
    assert_eq!(capture.stats.total_triangles, 12); // cube = 12 triangles

    // Verify pixels: cube should produce visible pixels.
    assert!(
        capture.visible_pixel_count > 0,
        "cube should produce visible pixels, got 0"
    );
    assert!(
        capture.coverage_percent() > 0.1,
        "coverage should be > 0.1%, got {:.2}%",
        capture.coverage_percent()
    );

    // Verify compact text contains expected data.
    let text = capture.to_compact_text();
    assert!(text.contains("320x240"));
    assert!(text.contains("1 batches"));
    assert!(text.contains("12 tris"));
    assert!(text.contains("COVERAGE"));

    // Verify pixel buffer size.
    assert_eq!(capture.pixels_rgba.len(), 320 * 240 * 4);

    println!("Capture diagnostic output:\n{text}");
}

#[test]
fn test_capture_empty_scene_no_visible_pixels() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 1.0, 0.1, 100.0),
    );

    let frame = Frame::new(camera);
    let mut target = GpuCaptureTarget::new(renderer.gpu.device(), 100, 100);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    assert_eq!(capture.visible_pixel_count, 0, "empty scene should have 0 visible pixels");
    assert_eq!(capture.stats.batch_count, 0);
    assert_eq!(capture.stats.total_triangles, 0);
}

#[test]
fn test_capture_two_cubes() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).expect("create_mesh");
    let red = renderer.create_material(Material::flat(0xFFFF0000));
    let green = renderer.create_material(Material::flat(0xFF00FF00));

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 2.0, 8.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 320.0 / 240.0, 0.1, 100.0),
    );

    let mut frame = Frame::new(camera);
    frame.draw(mesh_h, red, Mat4::translation(-2.0, 0.0, 0.0));
    frame.draw(mesh_h, green, Mat4::translation(2.0, 0.0, 0.0));

    let mut target = GpuCaptureTarget::new(renderer.gpu.device(), 320, 240);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    assert_eq!(capture.stats.batch_count, 2);
    assert_eq!(capture.stats.total_triangles, 24);
    assert!(capture.visible_pixel_count > 0, "two cubes should produce visible pixels");

    let text = capture.to_compact_text();
    assert!(text.contains("2 batches"));
    assert!(text.contains("24 tris"));
    println!("{text}");
}
```

**Step 2: Run the integration tests**

Run: `cargo test -p abrash-gpu-render --test capture_test`
Expected: PASS (3/3 tests, or SKIP if no GPU)

Note: the test accesses `renderer.gpu.device()` — if that field is `pub(crate)`, the
integration test (in `tests/`) can't access it. We need to expose a `device()` accessor
on `GpuRenderer`:

```rust
// Add to renderer.rs GpuRenderer impl block:
/// Access the underlying GPU device (for creating capture targets).
#[must_use]
pub fn device(&self) -> &wgpu::Device {
    self.gpu.device()
}
```

**Step 3: Run the integration tests again**

Run: `cargo test -p abrash-gpu-render --test capture_test`
Expected: PASS (3/3 tests)

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/tests/capture_test.rs crates/abrash-gpu-render/src/renderer.rs
git commit -m "test(gpu-render): capture integration tests — cube, empty scene, two cubes"
```

---

### Task 9: GpuSurface (Windowed Display, Feature-Gated)

**Files:**
- Modify: `crates/abrash-gpu-render/src/surface.rs`

**Context:** Wraps a wgpu surface created from a winit window. Feature-gated behind `windowed`. This is Track A — native window rendering with swapchain present, no pixel readback.

**Step 1: Write implementation**

```rust
// surface.rs
//! Windowed display surface (feature-gated behind `windowed`).
//!
//! Wraps a wgpu surface created from a winit window. The `GpuRenderer` renders
//! into this surface via `render_to_surface()`.

use crate::device::GpuDevice;
use std::sync::Arc;
use winit::window::Window;

/// A windowed display surface backed by a wgpu swapchain.
pub struct GpuSurface {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl GpuSurface {
    /// Create a surface from a winit window.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface cannot be created or configured.
    pub fn from_window(gpu: &GpuDevice, window: Arc<Window>) -> Result<Self, String> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err("window size must be non-zero".to_string());
        }

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|e| format!("failed to create surface: {e}"))?;

        let capabilities = surface.get_capabilities(&gpu.adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&gpu.device, &config);

        let (depth_texture, depth_view) =
            Self::create_depth_resources(&gpu.device, size.width, size.height);

        Ok(Self {
            surface,
            config,
            depth_texture,
            depth_view,
            width: size.width,
            height: size.height,
        })
    }

    /// Resize the surface (call when window size changes).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
        let (depth_texture, depth_view) = Self::create_depth_resources(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    /// Surface texture format (needed for pipeline creation).
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    fn create_depth_resources(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GpuSurface Depth"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}
```

**Step 2: Verify compilation with feature**

Run: `cargo check -p abrash-gpu-render --features windowed`
Expected: compiles

**Step 3: Verify compilation WITHOUT feature (surface module should be excluded)**

Run: `cargo check -p abrash-gpu-render --no-default-features`
Expected: compiles (no winit dependency)

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/surface.rs
git commit -m "feat(gpu-render): GpuSurface — windowed display surface, feature-gated behind 'windowed'"
```

---

### Task 10: GpuRenderer::render_to_surface()

**Files:**
- Modify: `crates/abrash-gpu-render/src/renderer.rs`

**Context:** Adds the windowed rendering path (Track A). Renders a `Frame` to a `GpuSurface` via wgpu swapchain — no pixel readback. Feature-gated behind `windowed`.

**Step 1: Add render_to_surface to GpuRenderer**

Add this `impl` block to `renderer.rs`, gated on `#[cfg(feature = "windowed")]`:

```rust
#[cfg(feature = "windowed")]
impl GpuRenderer {
    /// Create a GPU renderer for windowed rendering.
    ///
    /// The pipeline is created with the surface's texture format.
    /// Use this constructor when rendering to a window; use `new_headless()`
    /// when using `capture()` only.
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable GPU adapter is found.
    pub fn new_windowed(surface: &crate::surface::GpuSurface) -> Result<Self, String> {
        let config = GpuDeviceConfig::headless();
        let gpu = GpuDevice::new_headless(&config)?;

        let pipeline = MvpPipeline::new(&gpu.device, surface.format());

        let uniform_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GpuRenderer Uniform"),
                contents: bytemuck::bytes_of(&MvpUniform::new(&Mat4::identity(), 0xFFFFFFFF)),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GpuRenderer Uniform Bind Group"),
            layout: &pipeline.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            gpu,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            meshes: Vec::new(),
            materials: Vec::new(),
        })
    }

    /// Render a frame to a windowed surface.
    ///
    /// Acquires the next swapchain texture, renders all draw commands, and
    /// presents. No pixel readback — this is the fast display path.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface is lost or a handle is invalid.
    pub fn render_to_surface(
        &mut self,
        frame: &Frame,
        surface: &crate::surface::GpuSurface,
    ) -> Result<(), String> {
        let output = surface
            .surface
            .get_current_texture()
            .map_err(|e| format!("surface error: {e}"))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let view_proj = frame.camera.view * frame.camera.projection;
        let clear_color = frame
            .clear_color
            .map_or(wgpu::Color::BLACK, argb_to_wgpu_color);

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GpuRenderer Surface Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GpuRenderer Surface Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &surface.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            for (cmd_idx, cmd) in frame.commands.iter().enumerate() {
                let gpu_mesh = self
                    .meshes
                    .get(cmd.mesh.index as usize)
                    .and_then(|slot| slot.as_ref())
                    .ok_or_else(|| format!("stale mesh handle at command {cmd_idx}"))?;

                let gpu_mat = self
                    .materials
                    .get(cmd.material.index as usize)
                    .and_then(|slot| slot.as_ref())
                    .ok_or_else(|| format!("stale material handle at command {cmd_idx}"))?;

                let mvp = cmd.transform * view_proj;
                let uniform = MvpUniform::new(&mvp, gpu_mat.color);
                self.gpu.queue.write_buffer(
                    &self.uniform_buffer,
                    0,
                    bytemuck::bytes_of(&uniform),
                );

                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(
                    gpu_mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        self.gpu.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p abrash-gpu-render --features windowed`
Expected: compiles

**Step 3: Commit**

```bash
git add crates/abrash-gpu-render/src/renderer.rs
git commit -m "feat(gpu-render): GpuRenderer::render_to_surface() — windowed display path"
```

---

### Task 11: Windowed Cube Example

**Files:**
- Create: `examples/gpu_mvp_cube.rs`
- Modify: `Cargo.toml` (root, add example entry)

**Context:** Replaces the old yaw/pitch/distance shader with the new MVP-based `GpuRenderer` + `GpuSurface`. Demonstrates the complete windowed rendering path.

**Step 1: Write the example**

```rust
// examples/gpu_mvp_cube.rs
//! GPU MVP cube demo — renders a rotating cube using GpuRenderer + GpuSurface.
//!
//! This replaces the old yaw/pitch/distance shader with the proper MVP pipeline
//! from abrash-core/abrash-render.
//!
//! Run with: `cargo run --example gpu_mvp_cube --features gpu-render`

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_gpu_render::surface::GpuSurface;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("{e}"))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Abrash GPU MVP Cube")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280u32, 720))
            .build(&event_loop)
            .map_err(|e| format!("{e}"))?,
    );

    let mut renderer = GpuRenderer::new_windowed_from_window(window.clone())?;

    let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).map_err(|e| e)?;
    let mat_h = renderer.create_material(Material::flat(0xFFFF4444));

    let start = Instant::now();

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => elwt.exit(),
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let t = start.elapsed().as_secs_f32();
                let angle = t * 0.8;

                let camera = FrameCamera::new(
                    Mat4::look_at(
                        Vec3::new(0.0, 2.0, 5.0),
                        Vec3::ZERO,
                        Vec3::new(0.0, 1.0, 0.0),
                    ),
                    Mat4::perspective(1.0, 1280.0 / 720.0, 0.1, 100.0),
                );

                let mut frame = Frame::new(camera);
                frame.draw(mesh_h, mat_h, Mat4::rotation_y(angle));

                if let Err(e) = renderer.render_to_surface_current(&frame) {
                    eprintln!("render error: {e}");
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        })
        .map_err(|e| format!("{e}"))
}
```

Note: This example reveals that `GpuRenderer` needs a constructor that creates both the
device AND the surface from a window. The exact API shape will be refined during
implementation — the example serves as the acceptance test.

**Step 2: Add example to root Cargo.toml**

```toml
[[example]]
name = "gpu_mvp_cube"
path = "examples/gpu_mvp_cube.rs"
required-features = ["gpu-render"]
```

**Step 3: Verify compilation**

Run: `cargo check --example gpu_mvp_cube --features gpu-render`
Expected: may need API adjustments (windowed constructor). Fix as needed.

**Step 4: Run the example**

Run: `cargo run --example gpu_mvp_cube --features gpu-render`
Expected: window opens, red cube rotates

**Step 5: Commit**

```bash
git add examples/gpu_mvp_cube.rs Cargo.toml
git commit -m "feat(gpu-render): GPU MVP cube example using new GpuRenderer + GpuSurface"
```

---

### Task 12: Update Capability Matrix + Final Validation

**Files:**
- Modify: `docs/adr/005a-capability-matrix.md`

**Step 1: Update capability matrix**

Add GPU Renderer row to the feature support table and a Phase 5 implementation note:

Add to the table:
```markdown
| GPU flat shading (wgpu)    | ❌                 | ✅ (gpu-render) | ✅ (capture)   |
| GPU debug capture          | ❌                 | ❌              | ✅ (capture)   |
```

Add Phase 5 implementation notes:
```markdown
### Phase 5 — GPU Renderer (2026-03-17)
- `GpuRenderer::capture()` headless path with pixel readback + diagnostics ✅
- `GpuRenderer::render_to_surface()` windowed path with swapchain present ✅
- New MVP WGSL shader (mat4x4 uniform, row-vector convention) ✅
- `GpuMeshBuffer` converts Mesh → GPU vertex/index buffers ✅
- `GpuDebugCapture` compact text dump for LLM consumption ✅
- winit feature-gated behind `windowed` ✅
- ADR 007 documents design decisions ✅
- Integration tests: cube capture, empty scene, two cubes ✅
```

**Step 2: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: all existing tests pass + new gpu-render tests pass

**Step 3: Verify headless seam purity**

Run: `cargo tree -p abrash-gpu-render --no-default-features`
Expected: no winit in the tree (only wgpu, bytemuck, pollster, abrash-core, abrash-render)

**Step 4: Commit**

```bash
git add docs/adr/005a-capability-matrix.md
git commit -m "docs: update capability matrix with Phase 5 GPU renderer"
```

---

## Implementation Notes

### wgpu Readback Alignment
wgpu requires `bytes_per_row` to be a multiple of 256 (`COPY_BYTES_PER_ROW_ALIGNMENT`).
The `aligned_bytes_per_row()` function handles this. When reading back, rows must be
un-padded to produce a contiguous pixel array.

### Uniform Buffer Updates
Phase 5 uses `queue.write_buffer()` to update the MVP uniform per draw call. This is
simple but suboptimal for many draw calls (CPU-GPU sync point per write). Phase 6 can
improve this with:
- Dynamic uniform buffer with offsets
- Instance buffers for batched draws
- Indirect draw commands

### Row-Major Convention in WGSL
When abrash-core's row-major `Mat4` bytes are uploaded, WGSL interprets them as
column-major (transposed). The shader compensates with `vec4(pos, 1.0) * mvp`
(row-vector multiply). This is verified by the integration tests.

### Resource Pool Simplification
Phase 5 uses a simple `Vec<Option<T>>` for GPU resource pools instead of the
generational `ResourcePool<T>` used by `CpuRenderer`. This is intentional — GPU resources
have different lifecycle concerns (GPU buffers can't be partially recycled). Future phases
can adopt generational handles if needed.

### Existing Code Preservation
All existing `GpuMeshApp`, `GpuOffscreenBench`, `GpuDemoConfig`, etc. in `lib.rs` are
preserved. Existing examples (`gpu_cube`, `gpu_pyramid`, `gpu_obj`) continue to work
unchanged. The new API lives in separate modules alongside the existing code.
