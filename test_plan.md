1. **Explore the codebase:** Find out the exact structure of `crates/abrash-render/src/experimental/` and where to plug in the new feature. (Done)
2. **Create the benchmark first:** `benches/oil_paint_bench.rs` and add it to `Cargo.toml`. Run the benchmark to establish a baseline (it will fail because the feature doesn't exist yet, acting as the Red Phase for performance).
3. **Write the test (Red Phase):** Create `crates/abrash-render/src/experimental/oil_paint.rs` with a failing `test_oil_paint` unit test to verify basic output bounds and color behavior.
4. **Implement the feature (Green Phase):** Implement `apply_oil_paint` in `oil_paint.rs`. It will use an intensity-histogram-based algorithm, storing bins locally (e.g., array of size 256 for levels) to calculate the dominant intensity and average color within the sliding neighborhood window. Use `thread_local!` buffers to prevent allocation in hot loops. Add the module to `crates/abrash-render/src/experimental/mod.rs`.
5. **Create the demo:** Create `examples/oil_paint_demo.rs` to showcase the filter, and add it to `Cargo.toml`.
6. **Refactor and Optimize (Refactor Phase):** Profile and optimize `apply_oil_paint` by chunking rows or parallelizing via `rayon`. Rely on `thread_local!` structs inside parallel iterators.
7. **Document & Log:** Append learning to `.jules/nova.md` as required.
8. **Pre-commit Checks:** Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done. (e.g., `cargo clippy`, `cargo fmt`, `cargo test`).
9. **Submit:** Submit a PR named "🌟 Nova: Oil Paint Filter" with the necessary sections.
