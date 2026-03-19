# Cross-Platform Winit Host Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Win32 as the default native desktop backend with a thin cross-platform `winit` host in the root `abrash` crate, keep TUI as an explicit option, and make examples plug their own update/render logic into a trait-based window host without dictating what the app boots into.

**Architecture:** Add a new `backend-winit` root feature and make it the default desktop backend. Introduce `src/platform/winit.rs` with a small host API (`WindowApp`, `WindowHostConfig`, `run_windowed`) plus a `SoftwarePresenter` helper for CPU framebuffers. Migrate CPU examples from the synchronous `platform::Window` loop to the host trait, keep GPU examples on `winit`, and leave the "GPU by default" flip for a later phase once the windowed GPU path is less fragile.

**Tech Stack:** Rust 2024, winit 0.29.15, softbuffer 0.4.8, clap 4.5, crossterm 0.29, ratatui 0.30, existing `abrash-core`, `abrash-render`, and `abrash-gpu-render`

---

## Scope Guard

This phase changes the native windowing seam. It does **not**:
- make GPU the default renderer yet
- add a built-in app shell or startup scene
- redesign `abrash-render`
- create a new workspace crate

This phase **does**:
- make cross-platform desktop windowing the default backend
- keep TUI available behind an explicit flag
- move native host policy into a thin root-level `platform::winit` module
- prove that CPU examples and at least one GPU example fit the same host seam

---

### Task 1: ADR 008 for the Platform Host

**Files:**
- Create: `docs/adr/008-winit-platform-host.md`
- Modify: `docs/adr/005a-capability-matrix.md`

**Step 1: Write ADR 008**

Create `docs/adr/008-winit-platform-host.md` with these sections:

```markdown
# ADR 008: Root Winit Platform Host

## Status
Accepted

## Context

The root crate currently defaults to `backend-win32`, with `src/platform/win32.rs`
providing a synchronous `Window` abstraction and `src/main.rs` acting as a TUI launcher.
This blocks native desktop support on Linux/macOS and makes the root platform seam
Windows-first.

The GPU path already uses `winit` in `abrash-gpu-render`, so native window creation is
already solved there. The missing piece is a cross-platform root host seam for the rest of
the app and examples.

## Decision

- Add `backend-winit` to the root crate and make it the default native desktop backend.
- Keep `backend-tui` as an explicit alternate mode.
- Define the host API in the root crate under `src/platform/`.
- The host is a thin callback-based runner, not a scene shell or app menu.
- CPU presentation uses `softbuffer`.
- GPU stays opt-in in this phase; "GPU by default" is deferred until later cleanup.

## Consequences

- CPU examples stop depending on the Win32-only `Window` implementation.
- The root binary can stop assuming TUI as the primary user experience.
- `backend-win32` remains only where D3D12-specific code still needs it.
```

**Step 2: Update ADR 005a capability matrix**

Update `docs/adr/005a-capability-matrix.md`:
- replace the "Win32 window" row with "Winit desktop window"
- add a note that the root crate defaults to `backend-winit`
- add a phase note for the new host seam and TUI opt-in mode

**Step 3: Verify docs render cleanly**

Run:

```bash
cargo fmt --all
```

Expected: no rustfmt changes outside the usual file formatting.

**Step 4: Commit**

```bash
git add docs/adr/008-winit-platform-host.md docs/adr/005a-capability-matrix.md
git commit -m "docs: ADR 008 for root winit platform host"
```

---

### Task 2: Root Feature Topology and Dependency Wiring

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/platform/mod.rs`

**Step 1: Write the failing compile expectation down**

Acceptance criteria for this task:
- `cargo check` works with default features on desktop
- `cargo check --no-default-features --features backend-tui` still works
- `cargo check --bin abrash --features backend-tui` still works

**Step 2: Update root Cargo features**

Modify `Cargo.toml`:

```toml
[features]
default = ["backend-winit"]
backend-winit = ["dep:winit", "dep:softbuffer"]
backend-win32 = ["dep:windows-sys"]
backend-tui = ["dep:crossterm", "dep:ratatui", "abrash-render/backend-tui"]
gpu-render = ["dep:abrash-gpu-render", "dep:winit"]

[dependencies]
winit = { version = "0.29.15", optional = true }
softbuffer = { version = "0.4.8", optional = true }
```

Do **not** remove `backend-win32` yet; D3D12-specific code still depends on it.

**Step 3: Replace the platform module wiring**

Update `src/platform/mod.rs` so that:
- `backend-winit` exposes `pub mod winit;`
- the new public API re-exports host types from `platform::winit`
- `backend-win32` no longer owns the default `Window` alias
- compile-time errors mention `backend-winit`, `backend-tui`, and `backend-wasm`

Target shape:

```rust
#[cfg(feature = "backend-winit")]
pub mod winit;

#[cfg(feature = "backend-winit")]
pub use winit::{run_windowed, HostError, SoftwarePresenter, WindowApp, WindowHostConfig};

#[cfg(feature = "backend-tui")]
pub mod tui;

#[cfg(feature = "backend-win32")]
pub mod win32;
```

**Step 4: Run compile checks**

Run:

```bash
cargo check
cargo check --no-default-features --features backend-tui
cargo check --bin abrash --features backend-tui
```

Expected:
- default build pulls `winit` + `softbuffer`
- TUI-only build still compiles
- the root TUI launcher still compiles when requested explicitly

**Step 5: Commit**

```bash
git add Cargo.toml src/platform/mod.rs
git commit -m "feat(platform): add backend-winit as the default native backend"
```

---

### Task 3: Trait-Based Winit Host

**Files:**
- Create: `src/platform/winit.rs`
- Modify: `src/platform/mod.rs`

**Step 1: Write the failing test**

Add inline unit tests in `src/platform/winit.rs` for the config and timing helpers:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_host_config_default() {
        let config = WindowHostConfig::default();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert!(config.vsync);
    }

    #[test]
    fn test_frame_clock_dt_is_non_negative() {
        let mut clock = FrameClock::new();
        let dt = clock.tick();
        assert!(dt >= 0.0);
    }
}
```

**Step 2: Implement the host API**

Create `src/platform/winit.rs` with this core shape:

```rust
pub struct WindowHostConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for WindowHostConfig { ... }

pub enum HostError {
    EventLoop(String),
    Window(String),
    Present(String),
}

pub struct FrameClock { ... }

impl FrameClock {
    pub fn new() -> Self { ... }
    pub fn tick(&mut self) -> f32 { ... }
}

pub struct WindowContext<'a> {
    pub event_loop: &'a winit::event_loop::EventLoopWindowTarget<()>,
    pub window: std::rc::Rc<winit::window::Window>,
    pub dt_seconds: f32,
}

pub trait WindowApp {
    type Error: std::error::Error + Send + Sync + 'static;

    fn config(&self) -> WindowHostConfig;

    fn init(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn resize(&mut self, _ctx: WindowContext<'_>, _width: u32, _height: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, _event: &winit::event::WindowEvent) -> Result<(), Self::Error> {
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;
    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;
}

pub fn run_windowed<A: WindowApp>(mut app: A) -> Result<(), HostError> { ... }
```

Implementation rules:
- use `winit::event_loop::EventLoop::new()` and `EventLoop::run`
- keep rendering inside `WindowEvent::RedrawRequested`
- call `window.request_redraw()` from `Event::AboutToWait`
- call `app.update(...)` before `app.render(...)` on redraw
- map app errors into `HostError::Present(...)` or a dedicated app wrapper error

**Step 3: Run the focused tests**

Run:

```bash
cargo test --lib platform::winit
```

Expected: config/timing tests pass.

**Step 4: Run compile verification**

Run:

```bash
cargo check
```

Expected: root crate compiles with the new host module.

**Step 5: Commit**

```bash
git add src/platform/winit.rs src/platform/mod.rs
git commit -m "feat(platform): add trait-based winit host runner"
```

---

### Task 4: SoftwarePresenter for CPU Framebuffers

**Files:**
- Modify: `src/platform/winit.rs`
- Test: inline `#[cfg(test)]` module

**Context:** CPU examples still render into `Framebuffer`. `winit` creates the window, but it does not present raw pixel buffers. Use `softbuffer` as the cross-platform CPU presentation layer.

**Step 1: Write the failing pure helper tests**

Add tests for the pixel conversion helper so we can validate presentation logic without a GUI:

```rust
#[test]
fn test_argb_to_softbuffer_pixel() {
    assert_eq!(argb_to_softbuffer(0xFF11_2233), 0x0011_2233);
}

#[test]
fn test_argb_to_softbuffer_ignores_alpha() {
    assert_eq!(argb_to_softbuffer(0x8011_2233), 0x0011_2233);
}
```

**Step 2: Implement `SoftwarePresenter`**

Add to `src/platform/winit.rs`:

```rust
pub struct SoftwarePresenter {
    // Wrap the softbuffer context/surface types needed for the current
    // window and display handle.
    ...
}

impl SoftwarePresenter {
    pub fn new(
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        window: std::rc::Rc<winit::window::Window>,
    ) -> Result<Self, HostError> { ... }

    pub fn present(&mut self, framebuffer: &crate::framebuffer::Framebuffer) -> Result<(), HostError> { ... }
}

fn argb_to_softbuffer(argb: u32) -> u32 {
    argb & 0x00FF_FFFF
}
```

Implementation notes:
- resize the surface to the framebuffer dimensions before presenting
- copy pixels into `surface.buffer_mut()`
- keep presentation errors mapped through `HostError::Present`

**Step 3: Run focused tests**

Run:

```bash
cargo test --lib argb_to_softbuffer
```

Expected: both helper tests pass.

**Step 4: Run compile verification**

Run:

```bash
cargo check
```

Expected: softbuffer integration compiles on the current desktop target.

**Step 5: Commit**

```bash
git add src/platform/winit.rs
git commit -m "feat(platform): add softbuffer presenter for CPU framebuffers"
```

---

### Task 5: Migrate CPU Examples to `WindowApp`

**Files:**
- Modify: `examples/cube_3d.rs`
- Modify: `examples/lit_cube.rs`
- Modify: `examples/normal_mapping_demo.rs`
- Modify: `examples/obj_viewer.rs`
- Modify: `examples/particles.rs`
- Modify: `examples/skybox_demo.rs`
- Modify: `examples/heat_vision.rs`
- Modify: `examples/anaglyph_demo.rs`
- Modify: `examples/chromatic_aberration_demo.rs`
- Modify: `examples/directional_blur_demo.rs`
- Modify: `examples/pixel_sort_demo.rs`
- Modify: `examples/raytracer_demo.rs`
- Modify: `examples/ssao_demo.rs`
- Modify: `examples/vision_demo.rs`
- Modify: `examples/voronoi_demo.rs`
- Modify: `examples/cloth_demo.rs`
- Modify: `examples/jelly_demo.rs`
- Modify: `examples/edge_glow_demo.rs`

**Step 1: Convert one representative example first**

Use `examples/cube_3d.rs` as the pattern:
- replace `use abrash::platform::Window;` with `use abrash::platform::{run_windowed, SoftwarePresenter, WindowApp, WindowHostConfig};`
- move the old render loop state into a `CubeApp` struct
- implement `WindowApp` for `CubeApp`
- move framebuffer blit into `render()` using `SoftwarePresenter`

Target sketch:

```rust
struct CubeApp {
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    presenter: Option<SoftwarePresenter>,
    timestep: FixedTimestep,
    cube: Mesh,
    angle_x: f32,
    angle_y: f32,
}

impl WindowApp for CubeApp {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn config(&self) -> WindowHostConfig { ... }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.event_loop, ctx.window.clone())?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> { ... }
    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> { ... }
}
```

**Step 2: Compile the representative example**

Run:

```bash
cargo check --example cube_3d
```

Expected: compile passes before sweeping the rest.

**Step 3: Sweep the remaining examples**

Apply the same pattern to every file above that currently imports `abrash::platform::Window` or `abrash::platform::win32::Win32Window`.

Rules:
- do not add a built-in menu or app shell
- keep example behavior unchanged
- keep example banners/help text unchanged unless backend-specific wording is wrong
- for Nova examples, preserve existing `required-features = ["nova"]`

**Step 4: Compile the migrated examples**

Run:

```bash
cargo check --example cube_3d
cargo check --example lit_cube
cargo check --example normal_mapping_demo
cargo check --example obj_viewer
cargo check --example particles
```

Expected: representative CPU examples compile with the host trait.

**Step 5: Commit**

```bash
git add examples/
git commit -m "refactor(examples): migrate CPU examples to the winit WindowApp host"
```

---

### Task 6: Integrate the Host with the New GPU Path

**Files:**
- Modify: `examples/gpu_mvp_cube.rs`
- Optionally modify: `examples/gpu_cube.rs`
- Optionally modify: `examples/gpu_pyramid.rs`
- Optionally modify: `examples/gpu_obj.rs`

**Context:** The GPU path already uses `winit`, but it currently bypasses the root host seam. Prove the new host is actually renderer-agnostic by moving at least the new MVP example onto it.

**Step 1: Write the MVP example against `WindowApp`**

Modify `examples/gpu_mvp_cube.rs` so that:
- the example owns its own `GpuRenderer`, `GpuSurface`, and `Frame`
- the example implements `WindowApp`
- `render()` calls `GpuRenderer::render_to_surface(...)`
- `resize()` updates `GpuSurface`

**Step 2: Compile the GPU MVP example**

Run:

```bash
cargo check --example gpu_mvp_cube --features gpu-render
```

Expected: compiles using the root host seam plus existing GPU renderer.

**Step 3: Decide whether to migrate the legacy GPU demos now**

If `gpu_cube`, `gpu_pyramid`, and `gpu_obj` can be moved cheaply, migrate them too.
If not, leave them on their current direct `abrash_gpu_render` path and record the debt
in `docs/adr/008-winit-platform-host.md`.

**Step 4: Commit**

```bash
git add examples/gpu_mvp_cube.rs docs/adr/008-winit-platform-host.md
git commit -m "feat(platform): prove the winit host with the MVP GPU example"
```

---

### Task 7: Root CLI and TUI Flag

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the explicit `--tui` flag**

Update the CLI args:

```rust
#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short)]
    demo: Option<String>,

    #[arg(long, short)]
    list: bool,

    #[arg(long)]
    tui: bool,
}
```

**Step 2: Make TUI opt-in**

Refactor `main()` so that:
- `--tui` runs the current dashboard path
- `--list` still prints demos without opening a UI
- `--demo <name>` launches the demo without forcing TUI
- non-Windows desktop no longer auto-forces `backend-tui` for CPU demos

Update `demo_command()` and `run_demo()` accordingly:
- default path should use default features
- `--tui` path should add `--no-default-features --features backend-tui`
- GPU examples continue to use `--features gpu-render`

**Step 3: Verify the CLI still compiles**

Run:

```bash
cargo check --bin abrash --features backend-tui
```

Expected: TUI launcher still compiles, now behind explicit flag handling.

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): make TUI an explicit flag instead of the default path"
```

---

### Task 8: Final Validation

**Files:**
- Modify: `docs/adr/005a-capability-matrix.md`
- Modify: `docs/adr/008-winit-platform-host.md`

**Step 1: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected: no formatting errors.

**Step 2: Run compile matrix**

Run:

```bash
cargo check
cargo check --no-default-features --features backend-tui
cargo check --bin abrash --features backend-tui
cargo check --example cube_3d
cargo check --example lit_cube
cargo check --example gpu_mvp_cube --features gpu-render
```

Expected:
- default desktop build compiles on `backend-winit`
- TUI-only build still compiles
- the explicit TUI binary path still compiles
- representative CPU and GPU examples compile

**Step 3: Run tests**

Run:

```bash
cargo test --workspace
```

Expected: workspace tests pass; GUI behavior is compile-verified, not asserted by headless tests.

**Step 4: Manual smoke checks**

Run manually:

```bash
cargo run --example cube_3d
cargo run --example lit_cube
cargo run --example gpu_mvp_cube --features gpu-render
cargo run --bin abrash --features backend-tui -- --tui
```

Expected:
- CPU examples open a native desktop window on the current OS
- GPU MVP example still opens and renders
- TUI launcher still works only when explicitly requested

**Step 5: Commit**

```bash
git add Cargo.toml src/platform/ src/main.rs examples/ docs/adr/
git commit -m "feat(platform): add cross-platform winit host and make TUI opt-in"
```

---

## Implementation Notes

### Why `softbuffer`

`winit` gives us portable window creation and events, but it does not present CPU-rendered
pixel buffers. `softbuffer` is the smallest cross-platform helper that fits the current CPU
examples without forcing them onto `wgpu`.

### Why the host is callback-based

`winit 0.29.15` supports `pump_events`, but its own docs warn that rendering should stay inside
the `winit` callback for portability. The public host API should therefore be callback-driven,
not a fake synchronous `while window.is_open()` shim.

### CPU now, GPU default later

Do not couple "cross-platform windowing" and "GPU is the default renderer" into one patch set.
This phase lands the host seam and CPU presentation first. The later GPU-default flip can happen
after:
- surface loss/outdated recovery is typed instead of stringified
- `clear_color = None` semantics are fixed in the GPU path
- post-process integration exists for the windowed path

### Win32 is not fully dead yet

Keep `backend-win32` alive only where D3D12-specific code still needs it. The default user-facing
desktop backend should be `backend-winit`.
