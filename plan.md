1. **Refactor duplicate rendering logic into `encode_render_passes`**.
   - I see that `capture` and `render_to_surface` in `crates/abrash-gpu-render/src/renderer.rs` both share nearly identical logic for recording their rendering passes (Shadows, G-Buffer, RT Shadows, Refraction, Deferred, Skybox, TAA, Refraction Resolve, Composition, Tone Mapping).
   - This leads to large functions (triggering `clippy::too_many_lines`) and duplicates the render pipeline logic.
   - I will extract this shared sequence of encoder calls into a new private helper function, e.g., `encode_render_passes`, which takes the configured encoder, the target `wgpu::TextureView`, and the frame information, and performs all the passes.
   - This exactly addresses the Forge principle of "Extract large blocks of logic into small, named helper functions" and removes the `clippy::too_many_lines` exception.

2. **Run tests & pre-commit validation**.
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features`
   - `cargo fmt --all`
   - Ensure `clippy::too_many_lines` ignores can be removed.
   - Verify zero behavior change.

3. **Log learning to `.jules/forge.md`**.
   - Title: `[Extracted Shared Render Passes]`
   - Add note on how identical series of `wgpu::CommandEncoder` passes between headless capture and windowed rendering can be deduplicated.

4. **Submit PR**.
