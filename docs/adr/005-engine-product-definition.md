# ADR 005: Engine Product Definition

## Status
Accepted

## Context

Abrash is currently a single-crate software rasterizer with an algorithm-first public API
(fill_triangle_*, TileRenderer). To become embeddable in external projects (doom-rs, mercer),
it needs a stable, consumer-facing API that decouples rendering from presentation.

The current public surface exposes implementation details (EdgeWalker, scanline functions,
tile internals) rather than engine-level concepts (submit mesh, render frame, get pixels).

## Decision

### Product Definition

Abrash is a **software rasterization engine** that renders 3D scenes into caller-owned
pixel buffers. It is NOT a windowing system, game engine, or GPU abstraction layer.

### Supported Embed Modes

1. **Offscreen Render-to-Buffer**: Caller owns the Framebuffer. Engine renders into it.
   No windowing, no event loop, no platform code. This is the primary embed mode.

2. **Engine-Owned Viewport**: The root `abrash` host/demo crate creates a window or TUI host
   and manages the event loop. Used for standalone demos and local experiments.

3. **Benchmark/Headless**: Offscreen mode with no display. Used for criterion benchmarks
   and CI validation. No platform dependencies.

### Acceptance Criterion

If an external adapter needs to import anything from `abrash::platform`, the API seam is wrong.

### What Abrash Does NOT Provide

- Window management for embedded consumers (that's the host's job)
- GPU abstraction (wgpu/Vulkan/Metal — out of scope for the core engine)
- Asset pipeline (OBJ loading is a convenience, not a contract)
- ECS or scene graph (Scene is a thin submission helper, not an architecture)

## Consequences

- The public API will be reorganized around Renderer/RenderTarget/Frame/Handles
- Platform code (Win32/TUI/WASM) moves behind a feature gate, not exported by default
- Algorithm-level functions (fill_triangle_*) remain available but are internal to the engine
- External consumers depend only on abrash-core (types) and abrash-render (trait + CPU impl)
- Nova/experimental effects fold into `post_process` module or stay behind `nova` feature flag —
  no separate crate needed
- `embed-demo` is the reference integration for caller-owned pixel/depth buffers
