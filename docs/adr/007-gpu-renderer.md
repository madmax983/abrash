# ADR 007: GPU Renderer Design

## Status
Accepted

## Context

Phase 4 proved the engine API seam is correct: `embed-demo` depends only on `abrash-core`
+ `abrash-render` with zero platform dependencies. The `CpuRenderer` handles all rendering
via software rasterization through `TileRenderer`.

For real-time applications, GPU rendering is needed. The existing `abrash-gpu-render`
crate has a standalone wgpu demo (`GpuMeshApp`) with its own yaw/pitch/distance shader
that does not integrate with the engine's `Frame`/`Mesh`/`Mat4` types. We need a
`GpuRenderer` that consumes the same `Frame` type as `CpuRenderer`, enabling
backend-agnostic scene submission.

## Decision

### `GpuRenderer` does not implement the `Renderer` trait

The `Renderer` trait couples `render_frame` to `RenderTarget` (CPU-side framebuffer +
z-buffer). GPU rendering targets either a wgpu surface (windowed) or a wgpu texture
(headless capture). Forcing those through `RenderTarget` would require readback every
frame, which defeats the point of the GPU path.

Instead, `GpuRenderer` gets its own API:
- `render_to_surface(frame, surface)` for windowed display without readback
- `capture(frame, target)` for headless readback plus diagnostics

Both consume `Frame` from `abrash-render` so scene submission stays shared.

### Two output paths

Track A, windowed:
- wgpu surface created from a winit window
- swapchain present, no pixel readback
- feature-gated behind `windowed`

Track B, headless capture:
- offscreen `Rgba8Unorm` texture plus readback buffer
- 256-byte row alignment for `bytes_per_row`
- `GpuDebugCapture` with pixel data, frame stats, and compact text output
- no windowing dependency, usable in CI and agent workflows

### Row-major matrix convention

`abrash-core::math::Mat4` is `#[repr(C, align(16))] [[f32; 4]; 4]`, stored row-major and
used with row vectors (`v' = v * M`). When those bytes are uploaded, WGSL interprets them
as column-major. The shader therefore uses `vec4<f32>(pos, 1.0) * mvp`, which preserves
the intended transform without a transpose step.

### Compact diagnostics

`GpuDebugCapture` emits dense LLM-friendly text instead of raw pixel dumps or verbose JSON:

```text
FRAME 320x240 2 batches 24 tris 1.2ms
  B0 12t color=0xFFFF4444 visible=236px
  B1 12t color=0xFF4444FF visible=189px
COVERAGE 5.5% (4248/76800)
```

## Consequences

- CPU and GPU paths share `Frame` submission
- handles are backend-owned and not interchangeable between renderers
- headless capture works without a windowing stack
- legacy demo code remains available for backward compatibility
