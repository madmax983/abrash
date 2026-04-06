# Abrash Graphics Engine

**Abrash** is a software rasterization workspace written in Rust.
The actual engine surfaces are split by responsibility:

- `abrash-render`: headless/offscreen render API surface for embedding
- `abrash-core`: foundational math, buffers, geometry, and utility types
- root `abrash`: host/demo/meta crate that re-exports the workspace and optional platform backends

The goal is still the same: demystify the graphics pipeline by implementing every stage, from vertex transformation to pixel shading, in readable Rust.

## Philosophy 📜

> "The best way to learn how a car works is to build one from scratch."

Modern graphics APIs (Vulkan, DirectX 12) are powerful but complex black boxes. **Abrash** peels back the layers:
*   **Optional GPU acceleration:** CPU rasterization remains the core path, with opt-in GPU features (`gpu-render`, `gpu-binning`) for hardware-backed experiments.
*   **No obscure drivers:** The code is the documentation.
*   **No magic:** Every pixel on screen can be traced back to a specific line of code.

## Features ✨

*   **Software Rasterization:** Triangle filling with flat, Gouraud, and perspective-correct textured shading.
*   **3D Pipeline:** Complete vertex transformation pipeline (Model -> View -> Projection -> Clip Space -> Screen Space).
*   **Z-Buffering:** Pixel-perfect depth testing for correct visibility.
*   **Post-Processing:** Screen-space effects like SSAO, Depth of Field, and Bloom (see [`post_process`] module).
*   **Math Library:** Custom `Vec3` and `Mat4` implementations optimized for graphics (SIMD-ready logic).
*   **Platform Abstraction:**
    *   **Native Windows:** High-performance windowing using Win32.
    *   **TUI Backend:** Runs in your terminal for true cross-platform compatibility (Linux/macOS).

## Quick Start

Ensure you have Rust installed. Then pick the surface that matches the job.

### Headless / Embed Path

If you want to embed Abrash into another project, start with the engine crates:

```bash
cargo test -p abrash-render --lib
cargo run -p embed-demo
```

`embed-demo` is the reference for caller-owned pixel/depth buffers with no platform dependency.

Integration notes:

- [Bevy buffer bridge](docs/integration/bevy-buffer-bridge.md)
- [doom-rs buffer bridge](docs/integration/doom-rs-buffer-bridge.md)

### Host / Demo Path

The root `abrash` crate is the host/demo layer. The shipped terminal binary requires the TUI backend:

```bash
cargo run -p abrash --release --no-default-features --features backend-tui
```

### Examples

Explore different capabilities of the engine through the provided examples:

```bash
# A simple 3D wireframe cube (good for debugging transformations)
cargo run --release --example cube_3d

# A solid cube with simple lighting
cargo run --release --example lit_cube

# A textured rotating cube with perspective correction
cargo run --release --example textured_cube

# A hardware-accelerated rotating cube (wgpu)
cargo run --release --example gpu_cube --features gpu-render

# A hardware-accelerated pyramid mesh via generic GPU runner
cargo run --release --example gpu_pyramid --features gpu-render
```

GPU cube controls: drag left mouse to orbit, mouse wheel to zoom, arrows/WASD to orbit,
`Q`/`E` to zoom, `Space` to toggle auto-rotation, `R` to reset camera.

GPU renderer benchmarks:
```bash
cargo bench --bench gpu_render --features gpu-render
```

Engine comparison benchmarks (headless update loops for Bevy/Fyrox plus wgpu offscreen reference):
```bash
cargo bench --bench gpu_engine_compare --features gpu-engine-compare
```

If you want the root library surface without any backend glue, it now checks cleanly with:

```bash
cargo check -p abrash --no-default-features --lib
```

## Architecture

The engine follows a standard graphics pipeline architecture:

```text
[ Mesh Data ]
      ⬇
[ Vertex Processing (Math) ]  <-- (Model -> World -> View -> Clip Space)
      ⬇
[ Clipping ]                  <-- (Sutherland-Hodgman Algorithm)
      ⬇
[ Rasterization ]             <-- (Triangle Setup & Edge Walking)
      ⬇
[ Fragment Processing ]       <-- (Interpolation & Shading)
      ⬇
[ Depth Test (Z-Buffer) ]     <-- (Visibility Check)
      ⬇
[ Framebuffer ]               <-- (Pixel Storage 0xAARRGGBB)
```

### Coordinate System

*   **Handedness:** Right-Handed (Y-Up, X-Right, -Z-Forward).
*   **Matrices:** **Row-Major** storage.
*   **Transformations:** **Row-Vector** convention ($v \cdot M$).
    *   Vectors are rows: `[x, y, z, w]`
    *   Multiplication order: `v_prime = v * Scale * Rotation * Translation`

## Documentation

We believe that code is only as good as its explanation. All public APIs are fully documented with examples.

To generate and view the documentation locally:
```bash
cargo doc --no-deps --open
```

This will open your browser to the local API reference, where you can explore modules like [`rasterizer`](src/rasterizer), [`math`](src/math), and [`geometry`](src/geometry).

## License

MIT

## Miri Setup

Miri is useful for catching undefined behavior in unsafe and low-level code paths.
This project keeps `stable` as the default toolchain and runs Miri via `nightly`.

```bash
# One-time install
rustup toolchain install nightly
rustup component add miri --toolchain nightly

# One-time stdlib setup for Miri
cargo +nightly miri-setup

# Fast smoke run (recommended during development)
cargo +nightly miri-smoke

# Run library tests under Miri
cargo +nightly miri-test
```
