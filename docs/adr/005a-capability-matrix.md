# Capability Matrix (ADR 005 Supplement)

## Status
Proposed

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
| Win32 window               | ❌                 | ✅ (feature)    | ❌             |
| TUI display                | ❌                 | ✅ (feature)    | ❌             |
| WASM display               | ❌                 | ✅ (feature)    | ❌             |
| Event loop                 | ❌                 | ✅              | ❌             |
| GPU compute binning        | ✅ (feature)       | ✅ (feature)    | ✅ (feature)   |
| PPM/TGA export             | ✅                 | ✅              | ✅             |

## Crate Dependency Map (3-crate workspace)

```
abrash-core
  math, geometry, framebuffer, zbuffer, hiz_buffer, mesh, texture,
  obj_loader, clipping, culling, time, utils
         ↑
abrash-render
  render_api (Renderer trait, CpuRenderer, handles, Frame, Material)
  rasterizer (TileRenderer, fill_triangle_*, scanline functions)
  scene, post_process (incl. nova effects behind `nova` feature flag)
  particles, skybox, heat_vision, ascii, procedural
         ↑
abrash-demos  (root crate)
  platform (Win32/TUI/WASM backends — feature-gated)
  main.rs (TUI demo launcher)
  examples/
  benches/
```

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
# This shows no platform deps for the engine crates
cargo tree -p abrash-core --no-default-features
cargo tree -p abrash-render --no-default-features
```
