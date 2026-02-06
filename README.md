# Abrash Graphics Engine 🎻

**Abrash** is a software rasterization engine written in Rust, designed for educational purposes and retro-graphics enthusiasts. It pays homage to the software rendering techniques popularized by legends like Michael Abrash.

## Features ✨

*   **Software Rasterization:** Triangle filling with flat and lit shading.
*   **3D Pipeline:** Complete vertex transformation pipeline (Clip Space -> Screen Space).
*   **Z-Buffering:** Depth testing for correct visibility.
*   **Math Library:** Custom `Vec3` and `Mat4` implementations optimized for graphics.
*   **Platform Abstraction:** Native Windows windowing support, with a TUI backend for cross-platform compatibility.

## Quick Start 🚀

Ensure you have Rust installed. Clone the repository and run the examples!

### Running the Demo

The main binary runs a lit 3D cube demo:

```bash
cargo run --release
```

### Examples

Explore different capabilities of the engine through the provided examples:

```bash
# A 3D wireframe cube
cargo run --example cube_3d

# A solid cube with lighting (similar to main)
cargo run --example lit_cube

# WebAssembly compatible cube demo
cargo run --example wasm_cube_3d
```

> **Note:** Use `--release` for smooth performance, as software rendering is CPU-intensive!

## Architecture 🏛️

*   **`math`**: The mathematical foundation. Uses **Row-Vector** convention (`v * M`) and a Right-Handed coordinate system.
*   **`rasterizer`**: Low-level pixel drawing and 2D primitive filling.
*   **`framebuffer`**: Manages the pixel buffer (0xAARRGGBB format).
*   **`zbuffer`**: Manages the depth buffer.
*   **`platform`**: Window creation and event handling.

## License

MIT
