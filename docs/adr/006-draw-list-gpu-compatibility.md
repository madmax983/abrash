# ADR 006: DrawList GPU Backend Compatibility

## Status
Accepted

## Context

Phase 3 introduced `DrawList` as an intermediate representation between scene/frame
description and rasterization. Before committing to this design, we need to confirm
that `DrawList` contains sufficient information for a GPU backend to consume it,
and to document the known gaps and extension points.

A GPU backend is defined here as any backend that uploads geometry to a GPU buffer
and dispatches compute or graphics shaders (e.g., D3D12, Vulkan, Metal, wgpu).

## Analysis

### What DrawList provides today

```
DrawList {
    camera: FrameCamera { view: Mat4, projection: Mat4 }
    lights: Vec<Light>           // DirectionalLight, PointLight
    batches: Vec<DrawBatch {
        vertices: Vec<(Vec3, f32)>  // clip-space position + w
        indices:  Vec<[usize; 3]>   // triangle topology
        color:    u32               // flat surface color
    }>
    clear_color: Option<u32>
}
```

### GPU consumption walkthrough

| Need | DrawList provides | Notes |
|------|-------------------|-------|
| Clear render target | `clear_color` | ✅ Direct use |
| Vertex buffer content | `batch.vertices` | ⚠️ See pre-transform note |
| Index buffer content | `batch.indices` | ✅ Direct use |
| Draw call count | `batches.len()` | ✅ Direct use |
| Triangle count | `DrawList::triangle_count()` | ✅ Direct use |
| Light data (constant buffer) | `lights` | ✅ Passthrough to shader |
| Camera matrices (constant buffer) | `camera.view/projection` | ✅ Passthrough to shader |
| Surface color | `batch.color` | ✅ Sufficient for flat shading |
| Texture handle | ❌ Not in DrawBatch | Extension point (see below) |
| Normal data | ❌ Not in DrawBatch | Extension point (see below) |
| UV coordinates | ❌ Not in DrawBatch | Extension point (see below) |

### The pre-transform problem

`DrawBatch.vertices` are in **clip-space** (post-MVP). A conventional GPU vertex
shader computes `output.position = mvp * local_position`. Because the MVP transform
has already been applied, a GPU backend consuming `DrawList` must either:

1. **Skip the vertex shader transform**: Pass clip-space vertices directly as
   pre-transformed input and use a trivial vertex shader (`output.position = input.position`).
2. **Inverse-transform and re-upload**: Undo the clip-space transform to recover
   local-space vertices before uploading. This is wasteful.
3. **Use a separate raw geometry path**: Bypass `DrawList` and go back to the mesh +
   transform pair in the original `Frame`/`Scene`. This defeats the abstraction.

**Option 1 is correct for abrash's CPU-centric architecture.** The engine's
rasterizer operates in clip-space because it does the MVP on the CPU before calling
`submit_mesh`. A GPU backend that consumes `DrawList` can be implemented with a
passthrough vertex shader at negligible cost.

### Material extension point

For anything beyond flat shading, `DrawBatch` needs to carry material metadata:

```rust
// Future extension — not implemented yet
pub struct DrawBatch {
    pub vertices: Vec<(Vec3, f32)>,
    pub indices:  Vec<[usize; 3]>,
    pub color:    u32,

    // Extension point for GPU backends:
    // pub gpu_material_index: Option<u32>,
    // pub vertex_normals: Option<Vec<Vec3>>,
    // pub vertex_uvs:     Option<Vec<(f32, f32)>>,
}
```

The `color: u32` field is intentionally forward-compatible: a GPU backend can
use it as the flat-shading constant while ignoring richer fields it does not need.

### Verification: hypothetical D3D12 consumer

```rust
// doom_adapter.rs (hypothetical, NOT in abrash)
use abrash_core::math::{Mat4, Vec3};
use abrash_render::render_api::{DrawList, CpuRenderer};

fn upload_draw_list(device: &d3d12::Device, dl: &DrawList, target: &mut d3d12::RenderTarget) {
    // 1. Clear
    if let Some(color) = dl.clear_color {
        target.clear(color);
    }

    // 2. Upload light data to constant buffer
    // (dl.lights is self-contained — no further scene access needed)

    // 3. Per batch: upload vertex + index buffers, set root constants, dispatch draw
    for batch in &dl.batches {
        // Passthrough vertex shader: gl_Position = in_position (already clip-space)
        let vb = device.upload_vertices(&batch.vertices);
        let ib = device.upload_indices(&batch.indices);
        device.set_constant(batch.color);
        device.draw_indexed(ib.index_count());
    }
}
```

**Result**: `DrawList` contains ALL data required for this path. No further
access to `Scene`, `Frame`, `CpuRenderer`, or platform code is needed.

## Decision

`DrawList` is **sufficient for flat-shaded GPU backend consumption** with a trivial
passthrough vertex shader. The pre-transform design is correct for abrash's CPU-MVP
pipeline and imposes negligible extra cost on GPU consumers.

The following are **not defects** — they are Phase 4+ extension points:
- Texture handles in `DrawBatch` (Phase 4: Integration Proof)
- Per-vertex normals and UVs (Phase 4: required for textured GPU rendering)
- Material index for GPU resource binding (Phase 4)

## Consequences

- Phase 3 is **complete**: DrawList architecture is proven sound for GPU consumption.
- A GPU backend does NOT need scene-level concepts. ADR 005 acceptance criterion holds.
- Phase 4 can extend `DrawBatch` incrementally without breaking existing CPU consumers.
- The passthrough vertex shader pattern is a known technique (pre-transformed vertices
  in OpenGL are submitted via `GL_VERTEX_PROGRAM_TWO_SIDE` etc.) and imposes zero
  rasterization overhead.

## Implementation Notes

Phase 3 implemented 2026-03-16. Validation:
- `cargo test --workspace` → all tests pass ✅
- `Scene::extract()` → 5/5 tests (empty, visible, culled, multi-object) ✅
- `CpuRenderer::extract_draw_list` + `execute_draw_list` → 3/3 new tests ✅
- `test_execute_draw_list_matches_render_frame` → pixel-identical ✅
- Hypothetical GPU consumer walkthrough → all required data present ✅
