# Abrash Graphics Engine 🎻

**Abrash** is a software rasterization engine written in Rust, designed for educational purposes and retro-graphics enthusiasts. It pays homage to the software rendering techniques popularized by legends like Michael Abrash in his "Graphics Programming Black Book".

The goal is to demystify the GPU pipeline by implementing every stage—from vertex transformation to pixel shading—in pure, readable Rust.

## Philosophy 📜

> "The best way to learn how a car works is to build one from scratch."

Modern graphics APIs (Vulkan, DirectX 12) are powerful but complex black boxes. **Abrash** peels back the layers:
*   **No GPU acceleration:** Everything runs on the CPU.
*   **No obscure drivers:** The code is the documentation.
*   **No magic:** Every pixel on screen can be traced back to a specific line of code.

## Features ✨

*   **Software Rasterization:** Triangle filling with flat, Gouraud, and perspective-correct textured shading.
*   **3D Pipeline:** Complete vertex transformation pipeline (Model -> View -> Projection -> Clip Space -> Screen Space).
*   **Z-Buffering:** Pixel-perfect depth testing for correct visibility.
*   **Math Library:** Custom `Vec3` and `Mat4` implementations optimized for graphics (SIMD-ready logic).
*   **Platform Abstraction:**
    *   **Native Windows:** High-performance windowing using Win32.
    *   **TUI Backend:** Runs in your terminal for true cross-platform compatibility (Linux/macOS).

## Quick Start 🚀

Ensure you have Rust installed. Clone the repository and run the examples!

### Running the Demo

The main binary runs a lit 3D cube demo. Use `--release` for smooth performance!

```bash
cargo run --release
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
```

**Note for Linux/macOS users:**
The default backend uses Win32. To run on non-Windows systems, use the TUI backend:
```bash
cargo run --release --no-default-features --features backend-tui
```

## Architecture 🏛️

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
