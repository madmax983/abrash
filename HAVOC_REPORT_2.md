# 👺 Havoc Report

Havoc has swept through the codebase. The wreckage is minimal, but cracks were found.

## 1. 🐛 Critical Bug Fixed: Integer Overflow in Rasterizer

Fuzzing revealed a panic in `src/rasterizer.rs` when rasterizing geometry with extreme coordinates.

*   **Trigger**: Vertices with coordinates like `v0_y = -2.7e24` caused integer overflow during negation.
*   **Panic**: `attempt to negate with overflow` in `prepare_scanline` when `xs = i32::MIN`.
*   **Fix**: Applied safe negation `-(xs as f32)` instead of `-xs as f32` to handle the `i32::MIN` edge case.
*   **Status**: **FIXED**. Verified with `havoc_rasterizer_fuzz.rs`.

## 2. 🧨 API Fragility: Panics on Invalid Input

The library exposes public constructors that panic on invalid input instead of returning `Result`. This makes it trivial for a malicious user (or a clumsy developer) to crash the application.

*   **Vulnerability**: Denial of Service via Panic.
*   **Affected APIs**:
    *   `TileRenderer::new(width, height)`: Panics if `width` or `height` is 0.
    *   `Texture::new(width, height)`: Returns `Result` but allocation strategy for huge textures is aggressive.
*   **Demonstration**: `tests/havoc_api_abuse.rs` confirms these panics.

**Recommendation**: Change `TileRenderer::new` to return `Result<Self, &'static str>`.

## 3. 💥 Denial of Service via OOM

*   **Trigger**: `Texture::new(65536, 65536)`.
*   **Effect**: Attempts to allocate ~16GB of RAM. Causes immediate process abort (OOM).
*   **Mitigation**: Use `try_reserve` or limit texture dimensions to reasonable constants (e.g. 16K x 16K).

## 4. ⚠️ Future-Compatibility Warnings

The codebase relies on `unsafe` operations inside `unsafe fn` without explicit `unsafe {}` blocks.
*   **Risk**: Rust 2024 will make this a hard error.
*   **Location**: `src/rasterizer.rs`, `src/post_process.rs`, `src/math.rs`.
*   **Status**: Currently generates warnings.

## 5. 🛡️ Robustness Verified

The following components survived intense fuzzing:
*   **Rasterizer**: `fill_triangle_3d` handled `NaN`, `Infinity`, and subnormal coordinates without crashing (after fix).
*   **Texture Mapping**: `fill_triangle_textured` handled invalid UVs and degenerate triangles safely.
*   **Math**: `Mat4` and `Vec3` operations are robust against float anomalies.
*   **OBJ Loader**: Protected against infinite loops and basic vertex bombs (capped at 1M vertices).

## 6. 🧪 New Harnesses

Added the following havoc harnesses:
*   `tests/havoc_rasterizer_fuzz.rs`: Fuzzes triangle rasterization.
*   `tests/havoc_texture_fuzz.rs`: Fuzzes texture mapping with edge cases.
*   `tests/havoc_math_panics.rs`: Fuzzes math primitives.
*   `tests/havoc_api_abuse.rs`: Demonstrates API panics.

Run them with:
```bash
cargo test --test havoc_rasterizer_fuzz
cargo test --test havoc_texture_fuzz
cargo test --test havoc_math_panics
cargo test --test havoc_api_abuse
```
