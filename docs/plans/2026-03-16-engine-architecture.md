# Engine Architecture Refactoring Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Abrash from an algorithm-first library into an embeddable render engine with a stable, consumer-facing API that decouples scene submission from backend execution.

**Architecture:** Design the render API traits first (Renderer, RenderTarget, handles), then split the codebase into 4 workspace crates along the API boundaries. Decouple scene extraction into a backend-agnostic draw list, then prove embeddability with a real integration.

**Tech Stack:** Rust 2024, Cargo workspace, existing Framebuffer/ZBuffer/TileRenderer internals, thiserror for library errors.

---

## Phase 0: Product Definition

### Task 1: Write ADR 005 — "Finished Engine Product"

**Files:**
- Create: `docs/adr/005-engine-product-definition.md`

This ADR defines what "done" means for the engine. Without it, the crate split is decorative theater.

**Step 1: Write the ADR**

```markdown
# ADR 005: Engine Product Definition

## Status
Proposed

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

2. **Engine-Owned Viewport**: Engine creates a window and manages the event loop.
   Caller submits frames via the Renderer trait. Used for standalone demos.

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
```

**Step 2: Commit**

```bash
git add docs/adr/005-engine-product-definition.md
git commit -m "docs: ADR 005 — define engine product boundaries and embed modes"
```

---

### Task 2: Create Capability Matrix

**Files:**
- Create: `docs/adr/005a-capability-matrix.md`

This documents what features are available in each embed mode, so the crate split
has clear boundaries.

**Step 1: Write the capability matrix**

```markdown
# Capability Matrix (ADR 005 Supplement)

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
| Nova experimental effects  | ✅                 | ✅              | ✅             |
| OBJ loading                | ✅                 | ✅              | ✅             |
| Win32 window               | ❌                 | ✅ (feature)    | ❌             |
| TUI display                | ❌                 | ✅ (feature)    | ❌             |
| WASM display               | ❌                 | ✅ (feature)    | ❌             |
| Event loop                 | ❌                 | ✅              | ❌             |
| GPU compute binning        | ✅ (feature)       | ✅ (feature)    | ✅ (feature)   |
| PPM/TGA export             | ✅                 | ✅              | ✅             |

## Crate Dependency Map

```
abrash-core       (math, geometry, framebuffer, zbuffer, mesh, texture, hiz)
    ↑
abrash-render     (Renderer trait, CpuRenderer, TileRenderer, Scene, post-process)
    ↑                    ↑
abrash-demos      abrash-nova (experimental effects, feature-gated)
(examples, TUI launcher, platform backends)
```

## Key Constraint

`abrash-core` and `abrash-render` MUST NOT depend on any platform crate
(windows-sys, crossterm, ratatui, ratzilla). Platform code lives exclusively
in `abrash-demos` (or a future `abrash-platform` if consumers need it).
```

**Step 2: Commit**

```bash
git add docs/adr/005a-capability-matrix.md
git commit -m "docs: ADR 005a — capability matrix for embed modes and crate boundaries"
```

---

## Phase 1: Render API Design

The API is designed before any crate split. Everything lives in the existing single crate
under a new `src/render_api/` module. This lets us iterate on the API with full test
coverage before making structural changes.

### Task 3: Define RenderTarget

**Files:**
- Create: `src/render_api/mod.rs`
- Create: `src/render_api/target.rs`
- Modify: `src/lib.rs` (add `pub mod render_api;`)

RenderTarget bundles Framebuffer + ZBuffer + optional HiZBuffer into a single
type that gets passed to the renderer. This eliminates the 3-argument pattern
(`&mut fb, &mut zb, &mut hiz`) scattered across the codebase.

**Step 1: Write the failing test**

Create `src/render_api/mod.rs`:

```rust
//! Stable engine-facing render API.
//!
//! This module provides the consumer-facing abstractions for the Abrash engine.
//! External projects should depend on these types rather than the algorithm-level
//! functions in [`crate::rasterizer`].

pub mod target;

pub use target::RenderTarget;
```

Create `src/render_api/target.rs`:

```rust
//! Render target combining pixel and depth buffers.

use crate::framebuffer::Framebuffer;
use crate::hiz_buffer::HiZBuffer;
use crate::zbuffer::ZBuffer;

/// A render target combining pixel buffer, depth buffer, and optional Hi-Z pyramid.
///
/// This is the primary output surface for the renderer. External consumers create
/// a RenderTarget and pass it to [`Renderer::render_frame`].
///
/// # Examples
///
/// ```
/// use abrash::render_api::RenderTarget;
///
/// let mut target = RenderTarget::new(800, 600).unwrap();
/// target.clear(0xFF000000); // Clear to black
/// assert_eq!(target.width(), 800);
/// assert_eq!(target.height(), 600);
/// ```
pub struct RenderTarget {
    pub(crate) framebuffer: Framebuffer,
    pub(crate) zbuffer: ZBuffer,
    pub(crate) hiz: Option<HiZBuffer>,
}

impl RenderTarget {
    /// Create a new render target with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are invalid (exceed i32::MAX or overflow).
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        let framebuffer = Framebuffer::new(width, height)?;
        let zbuffer = ZBuffer::new(width, height)?;
        Ok(Self {
            framebuffer,
            zbuffer,
            hiz: None,
        })
    }

    /// Enable hierarchical z-buffer for occlusion culling.
    pub fn enable_hiz(&mut self) {
        if self.hiz.is_none() {
            self.hiz = Some(HiZBuffer::new(self.width(), self.height()));
        }
    }

    /// Clear both pixel and depth buffers.
    pub fn clear(&mut self, color: u32) {
        self.framebuffer.clear(color);
        self.zbuffer.clear();
    }

    /// Width of the render target in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.framebuffer.width()
    }

    /// Height of the render target in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.framebuffer.height()
    }

    /// Read-only access to the pixel buffer (for display or export).
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        self.framebuffer.as_slice()
    }

    /// Mutable access to the pixel buffer (for post-processing).
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        self.framebuffer.as_mut_slice()
    }

    /// Read-only access to the depth buffer.
    #[must_use]
    pub fn depths(&self) -> &[f32] {
        self.zbuffer.as_slice()
    }

    /// Access the underlying Framebuffer (for compatibility with existing code).
    #[must_use]
    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    /// Mutable access to the underlying Framebuffer.
    pub fn framebuffer_mut(&mut self) -> &mut Framebuffer {
        &mut self.framebuffer
    }

    /// Access the underlying ZBuffer.
    #[must_use]
    pub fn zbuffer(&self) -> &ZBuffer {
        &self.zbuffer
    }

    /// Mutable access to the underlying ZBuffer.
    pub fn zbuffer_mut(&mut self) -> &mut ZBuffer {
        &mut self.zbuffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_target_creation() {
        let target = RenderTarget::new(800, 600).unwrap();
        assert_eq!(target.width(), 800);
        assert_eq!(target.height(), 600);
        assert_eq!(target.pixels().len(), 800 * 600);
        assert_eq!(target.depths().len(), 800 * 600);
    }

    #[test]
    fn test_render_target_invalid_dimensions() {
        assert!(RenderTarget::new(0, 600).is_ok()); // zero is valid for framebuffer
        assert!(RenderTarget::new(i32::MAX as u32 + 1, 600).is_err());
    }

    #[test]
    fn test_render_target_clear() {
        let mut target = RenderTarget::new(10, 10).unwrap();
        target.clear(0xFFFF0000);
        assert_eq!(target.pixels()[0], 0xFFFF0000);
        assert!(target.depths()[0].is_infinite());
    }

    #[test]
    fn test_render_target_hiz() {
        let mut target = RenderTarget::new(100, 100).unwrap();
        assert!(target.hiz.is_none());
        target.enable_hiz();
        assert!(target.hiz.is_some());
        // Calling again is a no-op
        target.enable_hiz();
        assert!(target.hiz.is_some());
    }

    #[test]
    fn test_render_target_pixel_access() {
        let mut target = RenderTarget::new(10, 10).unwrap();
        target.pixels_mut()[0] = 0xDEADBEEF;
        assert_eq!(target.pixels()[0], 0xDEADBEEF);
    }
}
```

**Step 2: Add module to lib.rs**

Add after `pub mod scene;` (line 111):

```rust
pub mod render_api;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test --lib render_api -- --nocapture`
Expected: 5 tests pass

**Step 4: Commit**

```bash
git add src/render_api/mod.rs src/render_api/target.rs src/lib.rs
git commit -m "feat: add RenderTarget type bundling framebuffer + zbuffer + hiz"
```

---

### Task 4: Define Handle Types and Resource Pool

**Files:**
- Create: `src/render_api/handles.rs`
- Modify: `src/render_api/mod.rs` (add `pub mod handles;`)

Handles are opaque IDs into resource pools. The renderer owns the actual data;
consumers hold lightweight handles. This enables the renderer to manage GPU uploads,
caching, and lifetime tracking without exposing internals.

**Step 1: Write handles module**

Create `src/render_api/handles.rs`:

```rust
//! Opaque resource handles and typed resource pool.
//!
//! Handles are lightweight IDs that reference renderer-owned resources.
//! They are type-safe (MeshHandle can't be used where TextureHandle is expected)
//! and generation-checked to detect use-after-free.

use std::marker::PhantomData;

/// Generation counter to detect stale handles.
type Generation = u32;

/// A typed, generation-checked resource handle.
///
/// The `T` phantom type prevents mixing handle types at compile time.
/// The generation field detects use-after-free at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) index: u32,
    pub(crate) generation: Generation,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    /// Create a new handle (internal use only).
    pub(crate) fn new(index: u32, generation: Generation) -> Self {
        Self {
            index,
            generation,
            _marker: PhantomData,
        }
    }
}

/// Marker type for mesh resources.
#[derive(Debug)]
pub struct MeshResource;
/// Marker type for texture resources.
#[derive(Debug)]
pub struct TextureResource;
/// Marker type for material resources.
#[derive(Debug)]
pub struct MaterialResource;

/// Handle to a mesh uploaded to the renderer.
pub type MeshHandle = Handle<MeshResource>;
/// Handle to a texture uploaded to the renderer.
pub type TextureHandle = Handle<TextureResource>;
/// Handle to a material definition in the renderer.
pub type MaterialHandle = Handle<MaterialResource>;

/// A generational arena for storing resources behind handles.
///
/// Supports O(1) insert, lookup, and remove with generation-based
/// stale handle detection.
pub struct ResourcePool<T> {
    entries: Vec<PoolEntry<T>>,
    free_list: Vec<u32>,
}

enum PoolEntry<T> {
    Occupied { value: T, generation: Generation },
    Vacant { generation: Generation },
}

impl<T> ResourcePool<T> {
    /// Create a new empty resource pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Insert a resource and return its handle.
    pub fn insert(&mut self, value: T) -> Handle<T> {
        if let Some(index) = self.free_list.pop() {
            let entry = &mut self.entries[index as usize];
            let generation = match entry {
                PoolEntry::Vacant { generation } => *generation,
                PoolEntry::Occupied { .. } => unreachable!("free list pointed to occupied slot"),
            };
            *entry = PoolEntry::Occupied { value, generation };
            Handle::new(index, generation)
        } else {
            let index = self.entries.len() as u32;
            let generation = 0;
            self.entries.push(PoolEntry::Occupied { value, generation });
            Handle::new(index, generation)
        }
    }

    /// Look up a resource by handle. Returns None if handle is stale or invalid.
    #[must_use]
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.entries.get(handle.index as usize).and_then(|entry| {
            match entry {
                PoolEntry::Occupied { value, generation } if *generation == handle.generation => {
                    Some(value)
                }
                _ => None,
            }
        })
    }

    /// Mutable lookup by handle.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.entries
            .get_mut(handle.index as usize)
            .and_then(|entry| match entry {
                PoolEntry::Occupied {
                    value, generation, ..
                } if *generation == handle.generation => Some(value),
                _ => None,
            })
    }

    /// Remove a resource and return it. Increments generation to invalidate old handles.
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        match entry {
            PoolEntry::Occupied { generation, .. } if *generation == handle.generation => {
                let new_gen = *generation + 1;
                let old = std::mem::replace(entry, PoolEntry::Vacant { generation: new_gen });
                self.free_list.push(handle.index);
                match old {
                    PoolEntry::Occupied { value, .. } => Some(value),
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_type_safety() {
        // This test verifies that MeshHandle and TextureHandle are distinct types.
        // If this compiles, the phantom type parameter is working.
        let mesh: MeshHandle = Handle::new(0, 0);
        let tex: TextureHandle = Handle::new(0, 0);
        // These are equal in value but incompatible in type — can't be mixed.
        assert_eq!(mesh.index, tex.index);
    }

    #[test]
    fn test_pool_insert_and_get() {
        let mut pool: ResourcePool<String> = ResourcePool::new();
        let h1 = pool.insert("hello".to_string());
        let h2 = pool.insert("world".to_string());

        assert_eq!(pool.get(h1), Some(&"hello".to_string()));
        assert_eq!(pool.get(h2), Some(&"world".to_string()));
    }

    #[test]
    fn test_pool_remove_invalidates_handle() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h = pool.insert(42);

        assert_eq!(pool.remove(h), Some(42));
        // Handle is now stale
        assert_eq!(pool.get(h), None);
        // Double remove returns None
        assert_eq!(pool.remove(h), None);
    }

    #[test]
    fn test_pool_reuses_slots() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h1 = pool.insert(1);
        pool.remove(h1);

        let h2 = pool.insert(2);
        // Reused same index
        assert_eq!(h2.index, h1.index);
        // But different generation
        assert_ne!(h2.generation, h1.generation);
        // Old handle doesn't work
        assert_eq!(pool.get(h1), None);
        // New handle works
        assert_eq!(pool.get(h2), Some(&2));
    }

    #[test]
    fn test_pool_get_mut() {
        let mut pool: ResourcePool<String> = ResourcePool::new();
        let h = pool.insert("hello".to_string());

        if let Some(val) = pool.get_mut(h) {
            val.push_str(" world");
        }

        assert_eq!(pool.get(h), Some(&"hello world".to_string()));
    }
}
```

**Step 2: Update mod.rs**

Add to `src/render_api/mod.rs`:

```rust
pub mod handles;

pub use handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
```

**Step 3: Run tests**

Run: `cargo test --lib render_api -- --nocapture`
Expected: 10 tests pass (5 target + 5 handles)

**Step 4: Commit**

```bash
git add src/render_api/handles.rs src/render_api/mod.rs
git commit -m "feat: add generational handle types and resource pool for render API"
```

---

### Task 5: Define Material and ShadingMode

**Files:**
- Create: `src/render_api/material.rs`
- Modify: `src/render_api/mod.rs`

Materials describe HOW to shade a surface. This replaces the current pattern of
choosing between fill_triangle_flat/gouraud/phong/textured at the call site.

**Step 1: Write material module**

Create `src/render_api/material.rs`:

```rust
//! Material definitions describing how surfaces are shaded.

use crate::math::Vec3;
use crate::render_api::TextureHandle;

/// How a surface should be shaded.
///
/// This enum replaces the algorithm-first pattern of choosing between
/// fill_triangle_flat/gouraud/phong/textured at each call site. The renderer
/// dispatches to the appropriate rasterization path based on the shading mode.
#[derive(Debug, Clone)]
pub enum ShadingMode {
    /// Single color per triangle. Fastest mode.
    Flat {
        color: u32,
    },
    /// Per-vertex color interpolation.
    Gouraud,
    /// Per-pixel lighting with specular highlights.
    Phong {
        shininess: f32,
        specular_strength: f32,
    },
    /// Perspective-correct texture mapping.
    Textured {
        texture: TextureHandle,
    },
    /// Texture mapping with per-vertex color modulation.
    TexturedGouraud {
        texture: TextureHandle,
    },
    /// Physically-based rendering.
    Pbr {
        albedo: TextureHandle,
        roughness: f32,
        metallic: f32,
    },
    /// Environment/cubemap reflection.
    Reflection {
        texture: TextureHandle,
        reflectivity: f32,
    },
    /// Normal-mapped with per-pixel lighting.
    NormalMapped {
        diffuse: TextureHandle,
        normal_map: TextureHandle,
        shininess: f32,
    },
}

/// A material definition combining shading mode with rendering properties.
#[derive(Debug, Clone)]
pub struct Material {
    /// How this surface is shaded.
    pub shading: ShadingMode,
    /// Base color tint (multiplied with shading result). 0xAARRGGBB.
    pub color: u32,
    /// Whether this material is affected by lighting.
    pub receive_light: bool,
}

impl Material {
    /// Create a simple flat-colored material.
    #[must_use]
    pub fn flat(color: u32) -> Self {
        Self {
            shading: ShadingMode::Flat { color },
            color,
            receive_light: false,
        }
    }

    /// Create a Gouraud-shaded material.
    #[must_use]
    pub fn gouraud(color: u32) -> Self {
        Self {
            shading: ShadingMode::Gouraud,
            color,
            receive_light: true,
        }
    }

    /// Create a Phong-shaded material.
    #[must_use]
    pub fn phong(color: u32, shininess: f32, specular_strength: f32) -> Self {
        Self {
            shading: ShadingMode::Phong {
                shininess,
                specular_strength,
            },
            color,
            receive_light: true,
        }
    }

    /// Create a textured material.
    #[must_use]
    pub fn textured(texture: TextureHandle) -> Self {
        Self {
            shading: ShadingMode::Textured { texture },
            color: 0xFFFFFFFF,
            receive_light: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_api::handles::Handle;
    use std::marker::PhantomData;

    #[test]
    fn test_flat_material() {
        let mat = Material::flat(0xFFFF0000);
        assert_eq!(mat.color, 0xFFFF0000);
        assert!(!mat.receive_light);
        match mat.shading {
            ShadingMode::Flat { color } => assert_eq!(color, 0xFFFF0000),
            _ => panic!("Expected Flat shading"),
        }
    }

    #[test]
    fn test_phong_material() {
        let mat = Material::phong(0xFFFFFFFF, 32.0, 0.5);
        assert!(mat.receive_light);
        match mat.shading {
            ShadingMode::Phong {
                shininess,
                specular_strength,
            } => {
                assert_eq!(shininess, 32.0);
                assert_eq!(specular_strength, 0.5);
            }
            _ => panic!("Expected Phong shading"),
        }
    }

    #[test]
    fn test_textured_material() {
        let tex_handle: TextureHandle = Handle::new(0, 0);
        let mat = Material::textured(tex_handle);
        assert_eq!(mat.color, 0xFFFFFFFF);
        assert!(!mat.receive_light);
    }
}
```

**Step 2: Update mod.rs**

Add to `src/render_api/mod.rs`:

```rust
pub mod material;

pub use material::{Material, ShadingMode};
```

**Step 3: Run tests**

Run: `cargo test --lib render_api -- --nocapture`
Expected: 13 tests pass

**Step 4: Commit**

```bash
git add src/render_api/material.rs src/render_api/mod.rs
git commit -m "feat: add Material and ShadingMode types for render API"
```

---

### Task 6: Define DrawCommand, Light, and Frame

**Files:**
- Create: `src/render_api/frame.rs`
- Modify: `src/render_api/mod.rs`

A Frame is the unit of work submitted to the renderer each tick. It contains
the camera, lights, and draw commands. This is the Scene's output format.

**Step 1: Write frame module**

Create `src/render_api/frame.rs`:

```rust
//! Frame submission types — the contract between scene logic and the renderer.

use crate::math::{Mat4, Vec3};
use crate::render_api::{MaterialHandle, MeshHandle};

/// A directional light source (infinite distance, parallel rays).
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Normalized direction vector (points FROM light TO scene).
    pub direction: Vec3,
    /// Light color as 0xRRGGBB (no alpha).
    pub color: u32,
    /// Intensity multiplier (0.0 = off, 1.0 = normal).
    pub intensity: f32,
}

/// A point light source.
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    /// World-space position.
    pub position: Vec3,
    /// Light color as 0xRRGGBB.
    pub color: u32,
    /// Intensity.
    pub intensity: f32,
    /// Attenuation radius (light fades to zero at this distance).
    pub radius: f32,
}

/// Light source in the scene.
#[derive(Debug, Clone, Copy)]
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
}

/// Camera definition for a frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameCamera {
    /// View matrix (world -> view space).
    pub view: Mat4,
    /// Projection matrix (view -> clip space).
    pub projection: Mat4,
}

impl FrameCamera {
    /// Create a camera from view and projection matrices.
    #[must_use]
    pub fn new(view: Mat4, projection: Mat4) -> Self {
        Self { view, projection }
    }
}

/// A single draw command: render this mesh with this material at this transform.
#[derive(Debug, Clone, Copy)]
pub struct DrawCommand {
    /// Handle to the mesh to render.
    pub mesh: MeshHandle,
    /// Handle to the material to use.
    pub material: MaterialHandle,
    /// Model transform (local -> world space).
    pub transform: Mat4,
}

/// A complete frame to be rendered.
///
/// The Frame is the unit of work submitted to [`Renderer::render_frame`].
/// It is backend-agnostic — both CPU and GPU renderers consume the same Frame.
///
/// # Examples
///
/// ```
/// use abrash::render_api::frame::{Frame, FrameCamera, DrawCommand, Light, DirectionalLight};
/// use abrash::render_api::handles::Handle;
/// use abrash::math::{Mat4, Vec3};
///
/// let camera = FrameCamera::new(
///     Mat4::look_at(Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
///     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
/// );
///
/// let frame = Frame {
///     camera,
///     lights: vec![Light::Directional(DirectionalLight {
///         direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
///         color: 0xFFFFFF,
///         intensity: 1.0,
///     })],
///     commands: vec![], // Add draw commands here
///     clear_color: Some(0xFF000000),
/// };
/// ```
pub struct Frame {
    /// Camera for this frame.
    pub camera: FrameCamera,
    /// Active lights.
    pub lights: Vec<Light>,
    /// Draw commands (mesh + material + transform).
    pub commands: Vec<DrawCommand>,
    /// If Some, clear the render target to this color before rendering.
    /// If None, render over the existing contents.
    pub clear_color: Option<u32>,
}

impl Frame {
    /// Create an empty frame with the given camera.
    #[must_use]
    pub fn new(camera: FrameCamera) -> Self {
        Self {
            camera,
            lights: Vec::new(),
            commands: Vec::new(),
            clear_color: Some(0xFF000000),
        }
    }

    /// Add a draw command.
    pub fn draw(&mut self, mesh: MeshHandle, material: MaterialHandle, transform: Mat4) {
        self.commands.push(DrawCommand {
            mesh,
            material,
            transform,
        });
    }

    /// Add a light to the frame.
    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use crate::render_api::handles::Handle;

    fn test_camera() -> FrameCamera {
        FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 5.0, 10.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
        )
    }

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(test_camera());
        assert!(frame.commands.is_empty());
        assert!(frame.lights.is_empty());
        assert_eq!(frame.clear_color, Some(0xFF000000));
    }

    #[test]
    fn test_frame_add_draw_command() {
        let mut frame = Frame::new(test_camera());
        let mesh: MeshHandle = Handle::new(0, 0);
        let mat: MaterialHandle = Handle::new(1, 0);
        let transform = Mat4::translation(1.0, 2.0, 3.0);

        frame.draw(mesh, mat, transform);
        assert_eq!(frame.commands.len(), 1);
        assert_eq!(frame.commands[0].mesh, mesh);
        assert_eq!(frame.commands[0].material, mat);
    }

    #[test]
    fn test_frame_add_lights() {
        let mut frame = Frame::new(test_camera());
        frame.add_light(Light::Directional(DirectionalLight {
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: 0xFFFFFF,
            intensity: 1.0,
        }));
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(5.0, 5.0, 5.0),
            color: 0xFF0000,
            intensity: 2.0,
            radius: 10.0,
        }));
        assert_eq!(frame.lights.len(), 2);
    }
}
```

**Step 2: Update mod.rs**

Add to `src/render_api/mod.rs`:

```rust
pub mod frame;

pub use frame::{DrawCommand, Frame, FrameCamera, Light};
```

**Step 3: Run tests**

Run: `cargo test --lib render_api -- --nocapture`
Expected: 16 tests pass

**Step 4: Commit**

```bash
git add src/render_api/frame.rs src/render_api/mod.rs
git commit -m "feat: add Frame, DrawCommand, Light, and FrameCamera for render API"
```

---

### Task 7: Define Renderer Trait

**Files:**
- Create: `src/render_api/renderer.rs`
- Modify: `src/render_api/mod.rs`

The Renderer trait is the core engine contract. This is what external consumers
program against. It manages resources and renders frames.

**USER CONTRIBUTION POINT**: The Renderer trait's resource lifetime model is a meaningful
design choice. There are two valid approaches:

1. **Eager upload**: Resources are validated and processed on `create_*`. Faster render_frame,
   but create/destroy has latency.
2. **Deferred upload**: Resources are queued on `create_*` and processed lazily on first
   render_frame. Faster creation, but first frame has a spike.

The trait definition below uses eager upload (validate on create, return error immediately).
If you prefer deferred, change `create_mesh` to return `MeshHandle` without `Result` and
add a `fn flush_pending(&mut self)` method.

**Step 1: Write renderer trait**

Create `src/render_api/renderer.rs`:

```rust
//! The Renderer trait — core engine contract.

use crate::math::Vec3;
use crate::mesh::Mesh;
use crate::render_api::frame::Frame;
use crate::render_api::handles::{MaterialHandle, MeshHandle, TextureHandle};
use crate::render_api::material::Material;
use crate::render_api::target::RenderTarget;
use crate::texture::Texture;

/// Errors from renderer operations.
#[derive(Debug)]
pub enum RenderError {
    /// A handle referenced a resource that no longer exists.
    StaleHandle(&'static str),
    /// The mesh data was invalid (e.g., index out of bounds).
    InvalidMesh(String),
    /// The texture data was invalid.
    InvalidTexture(String),
    /// Internal renderer error.
    Internal(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleHandle(kind) => write!(f, "stale {kind} handle"),
            Self::InvalidMesh(msg) => write!(f, "invalid mesh: {msg}"),
            Self::InvalidTexture(msg) => write!(f, "invalid texture: {msg}"),
            Self::Internal(msg) => write!(f, "renderer error: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// The core renderer trait.
///
/// Implementations manage resource pools and render Frames into RenderTargets.
/// External consumers program against this trait, not against fill_triangle_*
/// or TileRenderer directly.
///
/// # Lifecycle
///
/// 1. Create renderer
/// 2. Upload resources (meshes, textures, materials)
/// 3. Each frame: build a `Frame`, call `render_frame`
/// 4. Read pixels from `RenderTarget` for display or export
/// 5. Clean up resources when done
pub trait Renderer {
    /// Upload a mesh and return a handle.
    ///
    /// The renderer takes ownership of the mesh data. The returned handle
    /// is used in DrawCommands to reference this mesh.
    fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, RenderError>;

    /// Upload a texture and return a handle.
    fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, RenderError>;

    /// Register a material and return a handle.
    fn create_material(&mut self, material: Material) -> Result<MaterialHandle, RenderError>;

    /// Render a frame into the render target.
    ///
    /// This is the primary entry point. The renderer:
    /// 1. Optionally clears the target
    /// 2. Builds the view-projection matrix from the camera
    /// 3. Performs frustum culling
    /// 4. Transforms and rasterizes visible geometry
    /// 5. Writes pixels and depth to the target
    fn render_frame(
        &mut self,
        frame: &Frame,
        target: &mut RenderTarget,
    ) -> Result<(), RenderError>;

    /// Release a mesh resource.
    fn destroy_mesh(&mut self, handle: MeshHandle);

    /// Release a texture resource.
    fn destroy_texture(&mut self, handle: TextureHandle);

    /// Release a material resource.
    fn destroy_material(&mut self, handle: MaterialHandle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::render_api::frame::FrameCamera;

    /// A null renderer that accepts all operations but renders nothing.
    /// Used to validate the trait contract.
    struct NullRenderer;

    impl Renderer for NullRenderer {
        fn create_mesh(&mut self, _mesh: &Mesh) -> Result<MeshHandle, RenderError> {
            Ok(MeshHandle::new(0, 0))
        }

        fn create_texture(&mut self, _texture: &Texture) -> Result<TextureHandle, RenderError> {
            Ok(TextureHandle::new(0, 0))
        }

        fn create_material(&mut self, _material: Material) -> Result<MaterialHandle, RenderError> {
            Ok(MaterialHandle::new(0, 0))
        }

        fn render_frame(
            &mut self,
            _frame: &Frame,
            _target: &mut RenderTarget,
        ) -> Result<(), RenderError> {
            Ok(())
        }

        fn destroy_mesh(&mut self, _handle: MeshHandle) {}
        fn destroy_texture(&mut self, _handle: TextureHandle) {}
        fn destroy_material(&mut self, _handle: MaterialHandle) {}
    }

    #[test]
    fn test_null_renderer_creates_resources() {
        let mut renderer = NullRenderer;
        let mesh = Mesh::cube(1.0);
        let handle = renderer.create_mesh(&mesh).unwrap();
        assert_eq!(handle.index, 0);
    }

    #[test]
    fn test_null_renderer_renders_frame() {
        let mut renderer = NullRenderer;
        let mut target = RenderTarget::new(100, 100).unwrap();
        let camera = FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 5.0, 10.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 1.0, 0.1, 100.0),
        );
        let frame = Frame::new(camera);
        assert!(renderer.render_frame(&frame, &mut target).is_ok());
    }

    #[test]
    fn test_render_error_display() {
        let err = RenderError::StaleHandle("mesh");
        assert_eq!(format!("{err}"), "stale mesh handle");

        let err = RenderError::InvalidMesh("index out of bounds".to_string());
        assert_eq!(format!("{err}"), "invalid mesh: index out of bounds");
    }
}
```

**Step 2: Update mod.rs**

Add to `src/render_api/mod.rs`:

```rust
pub mod renderer;

pub use renderer::{RenderError, Renderer};
```

**Step 3: Run tests**

Run: `cargo test --lib render_api -- --nocapture`
Expected: 19 tests pass

**Step 4: Commit**

```bash
git add src/render_api/renderer.rs src/render_api/mod.rs
git commit -m "feat: add Renderer trait — core engine contract for frame submission"
```

---

### Task 8: Implement CpuRenderer

**Files:**
- Create: `src/render_api/cpu_renderer.rs`
- Modify: `src/render_api/mod.rs`

This is the production implementation that wraps TileRenderer. It bridges the
new API to the existing rasterization code without rewriting any scanline logic.

**USER CONTRIBUTION POINT**: The flat-shading dispatch path is straightforward, but
how CpuRenderer decides between scanline vs tiled rendering is a real design choice:

- Option A: Always use TileRenderer (simpler, one code path)
- Option B: Use `should_use_tiled_rendering()` heuristic (current logic, resolution-dependent)
- Option C: Let the caller choose via a config flag

The implementation below uses Option A (always tiled) since TileRenderer already handles
both paths internally. Override in `CpuRendererConfig` if you want Option B/C.

**Step 1: Write CpuRenderer**

Create `src/render_api/cpu_renderer.rs`:

```rust
//! CPU software renderer implementing the Renderer trait.
//!
//! Bridges the render API to the existing TileRenderer / scanline rasterization.

use crate::mesh::Mesh;
use crate::render_api::frame::Frame;
use crate::render_api::handles::{MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
use crate::render_api::material::Material;
use crate::render_api::renderer::{RenderError, Renderer};
use crate::render_api::target::RenderTarget;
use crate::rasterizer::TileRenderer;
use crate::texture::Texture;

/// Stored mesh data for the CPU renderer.
struct CpuMesh {
    mesh: Mesh,
}

/// Configuration for the CPU renderer.
pub struct CpuRendererConfig {
    /// Enable Hi-Z occlusion culling on the tile renderer.
    pub enable_hiz: bool,
}

impl Default for CpuRendererConfig {
    fn default() -> Self {
        Self { enable_hiz: true }
    }
}

/// Software rasterizer implementing the [`Renderer`] trait.
///
/// Uses TileRenderer internally for cache-efficient rendering.
///
/// # Examples
///
/// ```
/// use abrash::render_api::{RenderTarget, Renderer};
/// use abrash::render_api::cpu_renderer::CpuRenderer;
/// use abrash::render_api::frame::{Frame, FrameCamera};
/// use abrash::render_api::material::Material;
/// use abrash::mesh::Mesh;
/// use abrash::math::{Mat4, Vec3};
///
/// let mut renderer = CpuRenderer::new(800, 600);
/// let mut target = RenderTarget::new(800, 600).unwrap();
///
/// // Upload resources
/// let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
/// let mat_h = renderer.create_material(Material::flat(0xFFFF0000)).unwrap();
///
/// // Build frame
/// let camera = FrameCamera::new(
///     Mat4::look_at(Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
///     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
/// );
/// let mut frame = Frame::new(camera);
/// frame.draw(mesh_h, mat_h, Mat4::identity());
///
/// // Render
/// renderer.render_frame(&frame, &mut target).unwrap();
/// ```
pub struct CpuRenderer {
    tile_renderer: TileRenderer,
    meshes: ResourcePool<CpuMesh>,
    textures: ResourcePool<Texture>,
    materials: ResourcePool<Material>,
}

impl CpuRenderer {
    /// Create a new CPU renderer for the given resolution.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let mut tile_renderer = TileRenderer::new(width, height);
        tile_renderer.enable_hiz();
        Self {
            tile_renderer,
            meshes: ResourcePool::new(),
            textures: ResourcePool::new(),
            materials: ResourcePool::new(),
        }
    }

    /// Create with custom configuration.
    #[must_use]
    pub fn with_config(width: u32, height: u32, config: CpuRendererConfig) -> Self {
        let mut tile_renderer = TileRenderer::new(width, height);
        if config.enable_hiz {
            tile_renderer.enable_hiz();
        }
        Self {
            tile_renderer,
            meshes: ResourcePool::new(),
            textures: ResourcePool::new(),
            materials: ResourcePool::new(),
        }
    }
}

impl Renderer for CpuRenderer {
    fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, RenderError> {
        // Validate indices are in bounds
        for (tri_idx, indices) in mesh.indices.iter().enumerate() {
            for &idx in indices {
                if idx >= mesh.vertices.len() {
                    return Err(RenderError::InvalidMesh(format!(
                        "triangle {tri_idx} has index {idx} but mesh only has {} vertices",
                        mesh.vertices.len()
                    )));
                }
            }
        }
        Ok(self.meshes.insert(CpuMesh { mesh: mesh.clone() }))
    }

    fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, RenderError> {
        Ok(self.textures.insert(texture.clone()))
    }

    fn create_material(&mut self, material: Material) -> Result<MaterialHandle, RenderError> {
        Ok(self.materials.insert(material))
    }

    fn render_frame(
        &mut self,
        frame: &Frame,
        target: &mut RenderTarget,
    ) -> Result<(), RenderError> {
        // 1. Clear if requested
        if let Some(color) = frame.clear_color {
            target.clear(color);
        }

        // 2. Compute view-projection
        let view_proj = frame.camera.view * frame.camera.projection;

        // 3. Begin tile renderer frame
        self.tile_renderer.begin_frame();

        // 4. Process each draw command
        for cmd in &frame.commands {
            let cpu_mesh = self
                .meshes
                .get(cmd.mesh)
                .ok_or(RenderError::StaleHandle("mesh"))?;
            let material = self
                .materials
                .get(cmd.material)
                .ok_or(RenderError::StaleHandle("material"))?;

            let mvp = cmd.transform * view_proj;
            let mesh = &cpu_mesh.mesh;

            // Transform vertices to clip space
            let transformed: Vec<_> = mesh
                .vertices
                .iter()
                .map(|v| mvp.transform_point(*v))
                .collect();

            // Submit to tile renderer with material color
            self.tile_renderer
                .submit_mesh(&mesh.indices, &transformed, material.color);
        }

        // 5. End frame (bin + rasterize + merge)
        self.tile_renderer
            .end_frame(&mut target.framebuffer, &mut target.zbuffer);

        Ok(())
    }

    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(handle);
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        self.textures.remove(handle);
    }

    fn destroy_material(&mut self, handle: MaterialHandle) {
        self.materials.remove(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::render_api::frame::FrameCamera;
    use crate::render_api::material::Material;

    fn test_camera() -> FrameCamera {
        FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
        )
    }

    #[test]
    fn test_cpu_renderer_lifecycle() {
        let mut renderer = CpuRenderer::new(800, 600);
        let mut target = RenderTarget::new(800, 600).unwrap();

        // Upload
        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF0000))
            .unwrap();

        // Render
        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());
        assert!(renderer.render_frame(&frame, &mut target).is_ok());

        // Verify something was drawn (center shouldn't be clear color)
        let center = target.pixels()[(300 * 800 + 400) as usize];
        // The cube should produce some non-black pixels in the center area
        // (exact pixel depends on projection, but it shouldn't all be black)

        // Cleanup
        renderer.destroy_mesh(mesh_h);
        renderer.destroy_material(mat_h);
    }

    #[test]
    fn test_cpu_renderer_stale_handle() {
        let mut renderer = CpuRenderer::new(100, 100);
        let mut target = RenderTarget::new(100, 100).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF0000))
            .unwrap();

        // Destroy the mesh
        renderer.destroy_mesh(mesh_h);

        // Try to render with stale handle
        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());
        let result = renderer.render_frame(&frame, &mut target);
        assert!(result.is_err());
    }

    #[test]
    fn test_cpu_renderer_invalid_mesh() {
        let mut renderer = CpuRenderer::new(100, 100);

        let mut bad_mesh = Mesh::new();
        bad_mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        bad_mesh.indices.push([0, 1, 2]); // indices 1 and 2 are out of bounds

        let result = renderer.create_mesh(&bad_mesh);
        assert!(result.is_err());
    }

    #[test]
    fn test_cpu_renderer_empty_frame() {
        let mut renderer = CpuRenderer::new(100, 100);
        let mut target = RenderTarget::new(100, 100).unwrap();

        // Render empty frame (just clear)
        let frame = Frame::new(test_camera());
        assert!(renderer.render_frame(&frame, &mut target).is_ok());

        // All pixels should be clear color (black)
        assert!(target.pixels().iter().all(|&p| p == 0xFF000000));
    }

    #[test]
    fn test_cpu_renderer_renders_visible_pixels() {
        let mut renderer = CpuRenderer::new(200, 200);
        let mut target = RenderTarget::new(200, 200).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF0000))
            .unwrap();

        let camera = FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 3.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 1.0, 0.1, 100.0),
        );

        let mut frame = Frame::new(camera);
        frame.draw(mesh_h, mat_h, Mat4::identity());
        renderer.render_frame(&frame, &mut target).unwrap();

        // Count non-black pixels — a cube at z=3 with fov=90 should cover a good chunk
        let non_black = target.pixels().iter().filter(|&&p| p != 0xFF000000).count();
        assert!(
            non_black > 100,
            "Expected visible pixels from cube, got {non_black}"
        );
    }
}
```

**Step 2: Update mod.rs to final state**

Replace `src/render_api/mod.rs` entirely:

```rust
//! Stable engine-facing render API.
//!
//! This module provides the consumer-facing abstractions for the Abrash engine.
//! External projects should depend on these types rather than the algorithm-level
//! functions in [`crate::rasterizer`].
//!
//! # Quick Start
//!
//! ```
//! use abrash::render_api::{RenderTarget, Renderer};
//! use abrash::render_api::cpu_renderer::CpuRenderer;
//! use abrash::render_api::frame::{Frame, FrameCamera};
//! use abrash::render_api::material::Material;
//! use abrash::mesh::Mesh;
//! use abrash::math::{Mat4, Vec3};
//!
//! // 1. Create renderer and target
//! let mut renderer = CpuRenderer::new(800, 600);
//! let mut target = RenderTarget::new(800, 600).unwrap();
//!
//! // 2. Upload resources
//! let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
//! let mat_h = renderer.create_material(Material::flat(0xFFFF0000)).unwrap();
//!
//! // 3. Build and render frame
//! let camera = FrameCamera::new(
//!     Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
//!     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
//! );
//! let mut frame = Frame::new(camera);
//! frame.draw(mesh_h, mat_h, Mat4::identity());
//! renderer.render_frame(&frame, &mut target).unwrap();
//!
//! // 4. Read pixels
//! let pixels: &[u32] = target.pixels();
//! ```

pub mod cpu_renderer;
pub mod frame;
pub mod handles;
pub mod material;
pub mod renderer;
pub mod target;

pub use frame::{DrawCommand, Frame, FrameCamera, Light};
pub use handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
pub use material::{Material, ShadingMode};
pub use renderer::{RenderError, Renderer};
pub use target::RenderTarget;
```

**Step 3: Run all tests**

Run: `cargo test --lib -- --nocapture`
Expected: All existing 58+ tests pass, plus ~24 new render_api tests

**Step 4: Run clippy**

Run: `cargo clippy --lib -- -D warnings`
Expected: No warnings

**Step 5: Commit**

```bash
git add src/render_api/
git commit -m "feat: implement CpuRenderer wrapping TileRenderer behind Renderer trait"
```

---

### Task 9: Integration Test — Render API Produces Identical Output to Direct TileRenderer

**Files:**
- Create: `tests/render_api_integration.rs`

This test proves the new API is a zero-regression wrapper. Same scene rendered
through CpuRenderer and directly through TileRenderer must produce pixel-identical output.

**Step 1: Write the integration test**

Create `tests/render_api_integration.rs`:

```rust
//! Integration test: CpuRenderer produces pixel-identical output to direct TileRenderer usage.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use abrash::render_api::cpu_renderer::CpuRenderer;
use abrash::render_api::frame::{Frame, FrameCamera};
use abrash::render_api::material::Material;
use abrash::render_api::target::RenderTarget;
use abrash::render_api::Renderer;
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 200;

fn camera_matrices() -> (Mat4, Mat4) {
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::ZERO,
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    (view, proj)
}

#[test]
fn test_cpu_renderer_matches_direct_tile_renderer() {
    let mesh = Mesh::cube(1.0);
    let color = 0xFFFF0000;
    let transform = Mat4::identity();
    let (view, proj) = camera_matrices();
    let view_proj = view * proj;
    let mvp = transform * view_proj;

    // --- Direct TileRenderer path ---
    let mut fb_direct = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zb_direct = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let mut tile_renderer = TileRenderer::new(WIDTH, HEIGHT);
    tile_renderer.enable_hiz();

    fb_direct.clear(0xFF000000);
    zb_direct.clear();
    tile_renderer.begin_frame();

    let transformed: Vec<_> = mesh.vertices.iter().map(|v| mvp.transform_point(*v)).collect();
    tile_renderer.submit_mesh(&mesh.indices, &transformed, color);
    tile_renderer.end_frame(&mut fb_direct, &mut zb_direct);

    // --- CpuRenderer path ---
    let mut renderer = CpuRenderer::new(WIDTH, HEIGHT);
    let mut target = RenderTarget::new(WIDTH, HEIGHT).unwrap();

    let mesh_h = renderer.create_mesh(&mesh).unwrap();
    let mat_h = renderer.create_material(Material::flat(color)).unwrap();

    let camera = FrameCamera::new(view, proj);
    let mut frame = Frame::new(camera);
    frame.draw(mesh_h, mat_h, transform);
    renderer.render_frame(&frame, &mut target).unwrap();

    // --- Compare pixel-by-pixel ---
    let direct_pixels = fb_direct.as_slice();
    let api_pixels = target.pixels();

    assert_eq!(direct_pixels.len(), api_pixels.len());

    let mut mismatches = 0;
    for (i, (&a, &b)) in direct_pixels.iter().zip(api_pixels.iter()).enumerate() {
        if a != b {
            mismatches += 1;
            if mismatches <= 5 {
                let x = i % WIDTH as usize;
                let y = i / WIDTH as usize;
                eprintln!("Pixel mismatch at ({x}, {y}): direct=0x{a:08X} api=0x{b:08X}");
            }
        }
    }

    assert_eq!(
        mismatches, 0,
        "CpuRenderer output differs from direct TileRenderer by {mismatches} pixels"
    );
}
```

**Step 2: Run the integration test**

Run: `cargo test --test render_api_integration -- --nocapture`
Expected: PASS with 0 pixel mismatches

**Step 3: Commit**

```bash
git add tests/render_api_integration.rs
git commit -m "test: pixel-identical integration test proving CpuRenderer matches TileRenderer"
```

---

## Phase 2: Crate Split (outline — refine after Phase 1 ships)

Phase 2 splits the single crate into a Cargo workspace with 4 members.
This is mechanical work guided by the API boundaries established in Phase 1.

> **Note:** Detailed file-level steps for Phase 2 should be written AFTER Phase 1
> is complete, since the exact API surface may evolve during implementation.
> The outline below captures the architectural intent.

### Task 10: Convert to Workspace — Create abrash-core

**Goal:** Extract math, geometry, framebuffer, zbuffer, hiz_buffer, mesh, texture,
obj_loader, clipping, culling into `crates/abrash-core`.

**Files:**
- Create: `crates/abrash-core/Cargo.toml`
- Create: `crates/abrash-core/src/lib.rs`
- Move: `src/{math,geometry,framebuffer,zbuffer,hiz_buffer,mesh,texture,obj_loader,clipping,culling}.rs` → `crates/abrash-core/src/`
- Modify: Root `Cargo.toml` (convert to workspace)

**Key decisions:**
- `abrash-core` has ZERO platform dependencies (no windows-sys, crossterm, ratatui)
- `abrash-core` has ZERO optional features except `parallel` (for rayon on culling)
- Edition 2024, same clippy lints as root
- Root crate re-exports `abrash-core` modules for backward compatibility

**Verification:**
- `cargo test -p abrash-core` passes all moved tests
- `cargo test` (root) still passes via re-exports
- `cargo clippy -p abrash-core` clean

### Task 11: Create abrash-render

**Goal:** Extract render_api, rasterizer, scene, post_process, particles, skybox,
heat_vision, ascii, procedural into `crates/abrash-render`.

**Files:**
- Create: `crates/abrash-render/Cargo.toml` (depends on `abrash-core`)
- Create: `crates/abrash-render/src/lib.rs`
- Move: `src/{render_api,rasterizer,scene,post_process,particles,skybox,heat_vision,ascii,procedural,time,utils}.rs` → `crates/abrash-render/src/`

**Key decisions:**
- `abrash-render` depends on `abrash-core` for types
- `abrash-render` has NO platform dependencies
- Feature `parallel` propagated from `abrash-core`
- The former GPU compute binning experiment was later removed from the workspace

**Verification:**
- `cargo test -p abrash-render` passes all moved tests
- Integration test (Task 9) still passes

### Task 12: Create abrash-nova

**Goal:** Extract experimental modules into `crates/abrash-nova`.

**Files:**
- Create: `crates/abrash-nova/Cargo.toml` (depends on `abrash-core`)
- Move: `src/experimental/` → `crates/abrash-nova/src/`

**Key decisions:**
- Depends on `abrash-core` only (framebuffer-level effects)
- Does NOT depend on `abrash-render` (no scene, no renderer)
- Former `nova` feature becomes "include this crate"
- All nova benchmarks move to `crates/abrash-nova/benches/`

### Task 13: Restructure Root as Workspace Facade

**Goal:** Root `abrash` crate becomes a thin workspace facade + demo launcher.

**Files:**
- Modify: Root `Cargo.toml` (workspace members, re-export dependencies)
- Modify: `src/lib.rs` (re-export all subcrates for backward compat)
- Move: `src/main.rs`, `src/platform/` stay in root (demo-only)
- Move: Examples stay in root (they need platform)

**Key constraint:** `cargo test` at workspace root runs all subcrate tests.

### Task 14: Verify Full Test Suite

Run: `cargo test --workspace`
Expected: All 58+ original tests pass, plus new render_api tests.

### Task 15: Verify All Benchmarks Compile

Run: `cargo bench --workspace --no-run`
Expected: All 40+ benchmarks compile. Benchmarks that need platform features
use `required-features` in root Cargo.toml.

---

## Phase 3: Draw List Extraction (outline)

### Task 16: Define DrawList Intermediate Representation

**Goal:** Create a backend-agnostic `DrawList` that captures the output of scene
extraction (culled, transformed geometry) without committing to a rasterization backend.

**Location:** `crates/abrash-render/src/render_api/draw_list.rs`

**Key type:**
```rust
pub struct DrawList {
    pub camera: FrameCamera,
    pub lights: Vec<Light>,
    pub batches: Vec<DrawBatch>,
}

pub struct DrawBatch {
    pub vertices: Vec<(Vec3, f32)>,  // clip-space + w
    pub indices: Vec<[usize; 3]>,
    pub material: MaterialHandle,
}
```

### Task 17: Scene Builds DrawList

**Goal:** `Scene::extract()` returns a `DrawList` instead of calling `TileRenderer` directly.
The existing `Scene::render()` becomes `scene.extract() → renderer.execute(draw_list)`.

### Task 18: CpuRenderer Consumes DrawList

**Goal:** `CpuRenderer::render_frame` internally builds a `DrawList` from the `Frame`,
then feeds it to TileRenderer. This proves the abstraction.

### Task 19: Verify GPU Backend Compatibility

**Goal:** Confirm that `DrawList` contains enough information for a hypothetical GPU
backend to consume it (transformed vertices, material handles, light data).

---

## Phase 4: Integration Proof (outline)

### Task 20: doom-rs Offscreen Adapter

**Goal:** Build a minimal adapter that renders into a caller-owned buffer using only
`abrash-core` and `abrash-render`. No platform imports.

**Acceptance criterion:** The adapter file has zero imports from `abrash::platform`.
If it needs platform code, the API seam is still wrong — go back to Phase 1.

**Shape of the adapter:**
```rust
// doom_adapter.rs (in doom-rs, NOT in abrash)
use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::{CpuRenderer, Frame, FrameCamera, Material, RenderTarget, Renderer};

pub struct AbrashBackend {
    renderer: CpuRenderer,
    target: RenderTarget,
}

impl AbrashBackend {
    pub fn new(width: u32, height: u32) -> Self { ... }
    pub fn render(&mut self, scene_data: &DoomScene) -> &[u32] {
        // Build Frame from doom-rs scene data
        // Call renderer.render_frame()
        // Return target.pixels()
    }
}
```

### Task 21: Validate Seam Purity

Run: `cargo tree -p doom-adapter --no-default-features`
Expected: No `windows-sys`, `crossterm`, `ratatui`, or `ratzilla` in the dependency tree.

---

## Summary Timeline

| Phase | Tasks | Estimated Duration |
|-------|-------|--------------------|
| 0: Product Definition | 1-2 | 1 week |
| 1: Render API Design | 3-9 | 2 weeks |
| 2: Crate Split | 10-15 | 3 weeks |
| 3: Draw List Extraction | 16-19 | 2-3 weeks |
| 4: Integration Proof | 20-21 | 2 weeks |
| **Total** | **21 tasks** | **10-13 weeks** |

## Open Questions (resolve during Phase 0)

1. Should `Renderer` be object-safe (`dyn Renderer`) or generic (`impl Renderer`)?
   Object-safe enables runtime backend switching; generic enables inlining.
2. Should `Frame` own the light/command data or borrow it? Owning is simpler;
   borrowing avoids per-frame allocation.
3. Where do the 40+ benchmarks land? Most benchmark internal rasterizer paths —
   they belong in `abrash-render`. Nova benchmarks go in `abrash-nova`.
