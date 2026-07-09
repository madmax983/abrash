1. **Optimize TileBins clear**
   - In `crates/abrash-render/src/rasterizer/tile.rs`, replace the `.fill(u32::MAX)` clear logic in `TileBins` with a generational index approach. This avoids O(N) operations when clearing tile bins every frame.
   - We will add `generations: Vec<u32>` and `current_generation: u32` fields to the `TileBins` struct.
   - When calling `clear()`, simply increment `current_generation`.
   - Update `push()`, `iter()`, and direct array access checks to compare generations.
2. **Update Bolt journal**
   - Pre-commit rule: log the optimization to `.jules/bolt.md` explaining the generational index approach for the tile bins.
3. **Verify**
   - Run tests (`cargo test --workspace --features "parallel nova backend-tui"`).
   - Ensure formatting and lints pass (`cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`).
   - Run benchmarks `tile_rendering_sorting` and `clear_rect_bench` to measure improvements.
4. **Pre-commit**
   - Follow pre-commit instructions using `pre_commit_instructions`.
5. **Submit**
   - Commit the changes and request user approval using the `submit` tool.
