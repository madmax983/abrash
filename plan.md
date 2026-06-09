1.  **Refactor `EdgeWalker` Structs:**
    *   Modify `PerspectiveTextureEdgeWalker` in `crates/abrash-render/src/rasterizer/texture.rs` to use composition with `crate::rasterizer::core::EdgeWalker`.
    *   Modify `GouraudEdgeWalker` in `crates/abrash-render/src/rasterizer/gouraud.rs` to use composition with `crate::rasterizer::core::EdgeWalker`.
    *   Modify `PbrEdgeWalker` in `crates/abrash-render/src/rasterizer/pbr.rs` to use composition with `crate::rasterizer::core::EdgeWalker`.
    *   Modify `PhongEdgeWalker` and `ShadowPhongEdgeWalker` and `PointLitPhongEdgeWalker` in `crates/abrash-render/src/rasterizer/phong.rs` to use composition with `crate::rasterizer::core::EdgeWalker`.
    *   Modify any other struct named `*EdgeWalker`.
    *   Update their `new`, `step`, and `step_n` methods accordingly. Update call sites where fields are accessed directly (e.g. `edge.x` to `edge.base.x` or add an accessor, or make `base` pub(crate)).

2.  **Verify and Test:**
    *   Run `cargo test --all-features` to ensure no logic was broken.
    *   Run `cargo clippy --all-targets --all-features -- -D warnings`.
    *   Run `cargo fmt --all`.

3.  **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.**

4.  **Submit PR**
