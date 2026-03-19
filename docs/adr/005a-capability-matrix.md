# Capability Matrix (ADR 005 Supplement)

## Status
Accepted

## Context

Companion to ADR 005. Documents which features are available in each embed mode and
establishes the crate dependency boundaries that prevent platform code from leaking
into the engine API.

## Embed Mode Feature Support

| Feature                    | Offscreen (Buffer) | Engine Viewport | Headless/Bench |
|----------------------------|--------------------|-----------------|----------------|
| Flat shading               | ✅                 | ✅              | ✅             |
| Gouraud shading            | ✅                 | ✅              | ✅             |
| Phong shading              | ✅                 | ✅              | ✅             |
| Texture mapping            | ✅                 | ✅              | ✅             |
| PBR shading                | ✅                 | ✅              | ✅             |
| Normal mapping             | ✅                 | ✅              | ✅             |
| Tile-based rendering       | ✅                 | ✅              | ✅             |
| Hi-Z occlusion culling     | ✅                 | ✅              | ✅             |
| Frustum culling            | ✅                 | ✅              | ✅             |
| Parallel rendering (Rayon) | ✅                 | ✅              | ✅             |
| Post-processing effects    | ✅                 | ✅              | ✅             |
| Nova effects (feature flag)| ✅                 | ✅              | ✅             |
| OBJ loading                | ✅                 | ✅              | ✅             |
| Winit desktop window       | ❌                 | ✅ (default)    | ❌             |
| TUI display                | ❌                 | ✅ (opt-in)     | ❌             |
| WASM display               | ❌                 | ✅ (feature)    | ❌             |
| Event loop                 | ❌                 | ✅              | ❌             |
| GPU flat shading (wgpu)    | ❌                 | ✅ (gpu-render) | ✅ (capture)   |
| GPU debug capture          | ❌                 | ❌              | ✅ (capture)   |
| PPM/TGA export             | ✅                 | ✅              | ✅             |

## Crate Dependency Map (4-crate workspace)

```
abrash-core
  math, geometry, framebuffer, zbuffer, hiz_buffer, mesh, texture,
  obj_loader, clipping, culling, time, utils
         ↑
abrash-render
  render_api (Renderer trait, CpuRenderer, handles, Frame, Material, DrawList)
  rasterizer (TileRenderer, fill_triangle_*, scanline functions)
  scene (Scene::extract() -> DrawList, Scene::render())
  post_process (incl. nova effects behind `nova` feature flag)
  particles, skybox, heat_vision, ascii, procedural
         ↑                          ↑
embed-demo (Phase 4 proof)     abrash-demos  (root crate)
  AbrashBackend                  platform (Win32/TUI/WASM — feature-gated)
  EmbedScene / EmbedDraw         main.rs (TUI demo launcher)
  NO platform deps               examples/
                                 benches/
```

`embed-demo` depends ONLY on `abrash-core` + `abrash-render`. It is the live proof
that the engine API seam is correct. See Phase 4 validation below.

## Nova Decision

Nova/experimental effects do NOT get a separate crate. Instead:

- **Framebuffer-level effects** (tilt-shift, CRT, kuwahara, swirl, vignette, chromatic
  aberration, etc.): fold into `post_process` module in `abrash-render`. These are
  pure `&mut [u32]` transforms with no scene/mesh dependency.
- **Physics/simulation** (cloth, jelly): stay behind `nova` feature flag in `abrash-render`.
- **Raytracer, procedural generation, SDF, voxels**: stay behind `nova` feature flag.

Rationale: nova effects are framebuffer post-processors at heart. They belong in
`post_process` semantically. A separate crate would create an artificial boundary for
code that has no meaningful independent consumers.

## Key Constraint

`abrash-core` and `abrash-render` MUST NOT depend on any platform crate:
- No `windows-sys`
- No `crossterm`
- No `ratatui`
- No `ratzilla`

Platform code lives exclusively in the root `abrash-demos` crate, behind feature flags.

## Validation

The seam is correct when:
```bash
# Engine crates have no platform deps
cargo tree -p abrash-core --no-default-features
cargo tree -p abrash-render --no-default-features

# Embed demo depends on nothing but the engine crates (Phase 4 proof)
cargo tree -p embed-demo --no-default-features
```

## Implementation Notes

### Phase 2 — 3-crate workspace (2026-03-16)
- `cargo tree -p abrash-core --no-default-features` → single node, no deps ✅
- `cargo tree -p abrash-render --no-default-features` → only abrash-core, no platform libs ✅
- All 513+ tests pass across workspace ✅

### Phase 3 — DrawList IR (2026-03-17)
- `DrawList` + `DrawBatch` added to `abrash-render::render_api::draw_list` ✅
- `Scene::extract() -> DrawList` — frustum cull + vertex transform, 5/5 tests ✅
- `CpuRenderer::extract_draw_list` + `execute_draw_list` — `render_frame` delegates through them ✅
- `test_execute_draw_list_matches_render_frame` → pixel-identical ✅
- ADR 006: DrawList is sufficient for GPU passthrough-vertex-shader backend ✅

### Phase 4 — Integration proof (2026-03-17)

`cargo tree -p embed-demo --no-default-features`:
```
embed-demo v0.1.0
├── abrash-core v0.1.0
└── abrash-render v0.1.0
    └── abrash-core v0.1.0
```

- Zero platform crates in tree ✅ (no windows-sys, crossterm, ratatui, ratzilla)
- ADR 005 acceptance criterion met: **no imports from `abrash::platform`** ✅
- `cargo run -p embed-demo` renders 8 frames of two cubes, exports PPM ✅
- 5/5 embed-demo unit tests pass ✅

### Phase 5 — GPU Renderer (2026-03-17)
- `GpuRenderer::capture()` headless path with pixel readback and diagnostics ✅
- `GpuRenderer::render_to_surface()` windowed path with swapchain present ✅
- MVP WGSL shader uses the engine matrix convention correctly ✅
- `GpuMeshBuffer` converts `Mesh` into GPU vertex and index buffers ✅
- `GpuDebugCapture` emits compact diagnostic text for agent/debug workflows ✅
- `winit` is feature-gated behind `windowed` inside `abrash-gpu-render` ✅
- ADR 007 documents the GPU renderer design decisions ✅
- Integration tests cover empty scene, single cube, and two-material capture ✅

### Phase 6 — Root Winit Host (2026-03-18)
- `backend-winit` is the root default desktop backend ✅
- `backend-tui` remains explicit opt-in via CLI flag ✅
- Root platform host is thin and callback-based, not an app shell ✅
- CPU framebuffer presentation uses `softbuffer` on `winit` ✅
- ADR 008 documents the platform host split ✅
