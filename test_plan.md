1. **Red Phase (Write failing tests)**
   - Create `crates/abrash-render/src/experimental/svg_export.rs`.
   - Write a unit test that creates a small `Framebuffer`, calls `export_svg`, and verifies the output SVG file contains the expected `<svg>` and `<rect>` tags with correct colors. The test will initially fail because the function will be missing or unimplemented.

2. **Green Phase (Make tests pass)**
   - Define the `SvgExporter` trait with the `export_svg` method.
   - Implement the `SvgExporter` trait for `abrash_core::framebuffer::Framebuffer`.
   - The implementation will iterate over the pixels and generate `<rect>` elements for each pixel, mapping the ARGB `u32` into `rgb(r,g,b)` and `fill-opacity`.

3. **Refactor Phase**
   - Refactor the code for clean formatting, ensuring `std::io::BufWriter` is used to efficiently write the massive amount of strings.
   - Add proper documentation (doc comments with `///`).

4. **Integration**
   - Expose the module in `crates/abrash-render/src/experimental/mod.rs`.
   - Verify it's correctly gated behind `#[cfg(feature = "nova")]` if necessary (though the whole `experimental` module requires the `nova` feature, so it might be automatic).

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `cargo clippy`, `cargo test`, and `cargo fmt`.

6. **Log and Submit**
   - Log the idea and fate into `.jules/nova.md`.
   - Submit the PR using the Nova persona format.
