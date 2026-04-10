# Headless Embed API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make caller-owned offscreen buffers the primary, real embed path for Abrash, while preserving owned-target convenience APIs and keeping platform/demo code outside the engine crates.

**Architecture:** Add a borrowed render-target abstraction in `abrash-render`, route `CpuRenderer` and `TileRenderer` through slice-based target writes, then update `embed-demo` and the root crate so the public product story matches the actual dependency boundary. Keep `RenderTarget` as a compatibility wrapper instead of the only way to render.

**Tech Stack:** Rust 2024 workspace, `abrash-core`, `abrash-render`, `embed-demo`, root `abrash` crate, `cargo test`, `cargo check`.

---

## Current Gap

The current architecture is close, but not finished:

- ADR 005 says the primary embed mode is “caller-owned pixel buffers”, but the actual code still renders into owned `RenderTarget` / `Framebuffer` / `ZBuffer` types.
- `embed-demo` proves “no platform deps”, but it still owns its own internal target instead of accepting host-owned buffers.
- The root `abrash` crate still reads like the main product even though it defaults to window/backend features and exports platform glue.

The plan below closes those gaps without rewriting the engine.

### Task 1: Introduce Borrowed Render Target In `abrash-render`

**Files:**
- Create: `crates/abrash-render/src/render_api/borrowed_target.rs`
- Modify: `crates/abrash-render/src/render_api/mod.rs`
- Modify: `crates/abrash-render/src/render_api/target.rs`
- Test: inline `#[cfg(test)]` in `crates/abrash-render/src/render_api/borrowed_target.rs`

**Step 1: Write the failing tests**

Add tests for:

- `borrowed_target_accepts_exact_length_slices`
- `borrowed_target_rejects_short_pixel_slice`
- `borrowed_target_rejects_short_depth_slice`
- `owned_render_target_can_borrow_mut`

Expected API shape:

```rust
pub struct BorrowedRenderTarget<'a> {
    pixels: &'a mut [u32],
    depths: &'a mut [f32],
    width: u32,
    height: u32,
}

impl<'a> BorrowedRenderTarget<'a> {
    pub fn new(
        width: u32,
        height: u32,
        pixels: &'a mut [u32],
        depths: &'a mut [f32],
    ) -> Result<Self, &'static str>;
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-render borrowed_target -- --nocapture`

Expected: FAIL because `borrowed_target.rs` and `BorrowedRenderTarget` do not exist yet.

**Step 3: Write minimal implementation**

Implement:

- `BorrowedRenderTarget<'a>` with `new`, `width`, `height`, `pixels`, `pixels_mut`, `depths`, `depths_mut`
- length validation against `width * height`
- `RenderTarget::borrow_mut(&mut self) -> BorrowedRenderTarget<'_>`

Do **not** invent a trait here. Keep phase 1 concrete and boring.

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-render borrowed_target -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/abrash-render/src/render_api/borrowed_target.rs crates/abrash-render/src/render_api/mod.rs crates/abrash-render/src/render_api/target.rs
git commit -m "feat: add borrowed render target API"
```

### Task 2: Teach `CpuRenderer` To Render Into Borrowed Targets

**Files:**
- Modify: `crates/abrash-render/src/render_api/cpu_renderer.rs`
- Modify: `crates/abrash-render/src/render_api/target.rs`
- Test: inline `#[cfg(test)]` in `crates/abrash-render/src/render_api/cpu_renderer.rs`

**Step 1: Write the failing tests**

Add tests for:

- `test_execute_draw_list_into_borrowed_target_matches_owned_target`
- `test_render_frame_into_borrowed_target_matches_owned_target`

Use the existing `RenderTarget` path as the reference output and compare pixel/depth buffers against a borrowed-slice target.

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-render borrowed_target_matches_owned_target -- --nocapture`

Expected: FAIL because `CpuRenderer` only accepts `RenderTarget`.

**Step 3: Write minimal implementation**

Add:

```rust
pub fn execute_draw_list_into(
    &mut self,
    draw_list: &DrawList,
    target: &mut BorrowedRenderTarget<'_>,
);

pub fn render_frame_into(
    &mut self,
    frame: &Frame,
    target: &mut BorrowedRenderTarget<'_>,
) -> Result<(), RenderError>;
```

Then keep compatibility by rewriting the current methods as wrappers:

- `execute_draw_list(..., target: &mut RenderTarget)` should call `target.borrow_mut()`
- `render_frame(..., target: &mut RenderTarget)` should call `render_frame_into`

Do **not** break the existing owned-target call sites in this step.

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-render borrowed_target_matches_owned_target -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/abrash-render/src/render_api/cpu_renderer.rs crates/abrash-render/src/render_api/target.rs
git commit -m "feat: render frames into borrowed targets"
```

### Task 3: Factor `TileRenderer` To Write Into Slices Instead Of Concrete Buffer Types

**Files:**
- Modify: `crates/abrash-render/src/rasterizer/tile.rs`
- Test: inline `#[cfg(test)]` in `crates/abrash-render/src/rasterizer/tile.rs`

**Step 1: Write the failing tests**

Add tests for:

- `test_end_frame_into_slices_matches_end_frame`
- `test_render_batch_into_slices_matches_render_batch`
- `test_render_batch_textured_into_slices_matches_render_batch_textured`

Use one `Framebuffer`/`ZBuffer` pair as the reference path and one plain `(Vec<u32>, Vec<f32>)` pair as the borrowed-slice path.

**Step 2: Run test to verify it fails**

Run: `cargo test -p abrash-render into_slices_matches -- --nocapture`

Expected: FAIL because `TileRenderer` currently requires `Framebuffer` and `ZBuffer`.

**Step 3: Write minimal implementation**

Refactor `tile.rs` so the core merge/write path accepts raw slices plus dimensions:

```rust
pub fn end_frame_into_slices(
    &mut self,
    width: u32,
    height: u32,
    pixels: &mut [u32],
    depths: &mut [f32],
);
```

Then:

- keep `end_frame(&mut Framebuffer, &mut ZBuffer)` as a wrapper
- keep `render_batch(...)` and `render_batch_textured(...)` as wrappers
- share validation and merge logic in one internal helper

Do **not** genericize the entire rasterizer with traits. Slice-first helpers are enough.

**Step 4: Run test to verify it passes**

Run: `cargo test -p abrash-render into_slices_matches -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/abrash-render/src/rasterizer/tile.rs
git commit -m "refactor: route tile renderer through slice-based targets"
```

### Task 4: Make `embed-demo` Prove Caller-Owned Buffers, Not Just Platform Independence

**Files:**
- Modify: `crates/embed-demo/src/lib.rs`
- Modify: `crates/embed-demo/src/main.rs`
- Test: inline `#[cfg(test)]` in `crates/embed-demo/src/lib.rs`

**Step 1: Write the failing tests**

Add tests for:

- `test_render_into_external_buffers_produces_visible_pixels`
- `test_render_into_external_buffers_preserves_host_ownership`
- `test_render_into_external_buffers_matches_render_convenience_path`

**Step 2: Run test to verify it fails**

Run: `cargo test -p embed-demo external_buffers -- --nocapture`

Expected: FAIL because `AbrashBackend` currently renders only into its internal `RenderTarget`.

**Step 3: Write minimal implementation**

Add a primary embed API:

```rust
pub fn render_into(
    &mut self,
    scene: &EmbedScene<'_>,
    pixels: &mut [u32],
    depths: &mut [f32],
) -> Result<(), RenderError>;
```

Implementation notes:

- `render_into` should build a `BorrowedRenderTarget` and call `CpuRenderer::render_frame_into`
- keep `render()` only as a convenience wrapper for examples/tests
- change the crate-level docs and `main.rs` example text to lead with host-owned buffers first

**Step 4: Run test to verify it passes**

Run: `cargo test -p embed-demo external_buffers -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/embed-demo/src/lib.rs crates/embed-demo/src/main.rs
git commit -m "feat: add caller-owned buffer path to embed demo"
```

### Task 5: Make The Root Crate Stop Lying About What The Product Is

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/platform/mod.rs`
- Modify: `README.md`
- Modify: `docs/adr/005-engine-product-definition.md`

**Step 1: Write the failing verification command**

The root library should compile without forcing a platform backend when used as a library surface.

Run:

```bash
cargo check -p abrash --no-default-features --lib
```

Expected today: FAIL because `src/platform/mod.rs` hard-errors when no backend is selected.

**Step 2: Implement the minimal surface cleanup**

Make these changes:

- gate `pub mod platform;` behind backend features in `src/lib.rs`
- move the “no backend selected” hard failure out of library compilation and into the binary/demo path
- update `README.md` to say:
  - `abrash-render` is the embed/render API surface
  - `abrash-core` is foundational types/utilities
  - root `abrash` is the host/demo/meta crate
- update ADR 005 status from `Proposed` to `Accepted` only if the code now matches the decision

Do **not** rename packages in this phase. Naming churn can wait until the API truth is fixed.

**Step 3: Verify library and demo modes**

Run:

```bash
cargo check -p abrash --no-default-features --lib
cargo check -p abrash --features backend-winit --lib
cargo check -p embed-demo --no-default-features
```

Expected: all PASS

**Step 4: Commit**

```bash
git add Cargo.toml src/lib.rs src/platform/mod.rs README.md docs/adr/005-engine-product-definition.md
git commit -m "docs: align root crate surface with headless engine architecture"
```

### Task 6: Add Regression Proofs For The Two Real Consumers

**Files:**
- Create: `docs/integration/bevy-buffer-bridge.md`
- Create: `docs/integration/doom-rs-buffer-bridge.md`
- Modify: `README.md`

**Step 1: Write the verification checklist**

Document the exact host-side contract:

- pixel format: `0xAARRGGBB`
- depth format: `f32`
- dimensions must match renderer dimensions
- host owns lifetime/allocation of both slices
- renderer does not touch presentation/event-loop concerns

**Step 2: Add integration notes**

For Bevy:

- show how to allocate host `Vec<u32>` / `Vec<f32>`
- render via `render_into`
- upload color buffer into a Bevy image/texture

For `doom-rs`:

- show how to borrow Doom’s screen/depth storage
- render directly into it without extra intermediate ownership

This is documentation-first proof; do not block the plan on external repo changes.

**Step 3: Verify docs match reality**

Run:

```bash
cargo test -p embed-demo
cargo test -p abrash-render --lib
```

Expected: PASS, and docs refer only to APIs that actually exist.

**Step 4: Commit**

```bash
git add docs/integration/bevy-buffer-bridge.md docs/integration/doom-rs-buffer-bridge.md README.md
git commit -m "docs: add host-owned buffer integration guides"
```

## Verification Matrix

Run all of these before calling the roadmap complete:

```bash
cargo test -p abrash-render --lib
cargo test -p embed-demo
cargo check -p abrash --no-default-features --lib
cargo check -p abrash --features backend-winit --lib
cargo tree -p abrash-render --no-default-features
cargo tree -p embed-demo --no-default-features
```

Expected outcomes:

- `abrash-render` still has no platform dependencies
- `embed-demo` still depends only on `abrash-core` + `abrash-render`
- host-owned rendering is the first-class embed path
- owned `RenderTarget` remains as compatibility sugar

## Follow-Up Backlog (Not Part Of This Plan)

- Canonicalize quaternion API and remove duplicate public `Quat`
- Decide whether `sdf` stays in `abrash-core` or becomes `abrash-sdf`
- Consider renaming the root crate/package to make the engine-vs-demos split obvious

