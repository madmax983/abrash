1. **Create Benchmark:** Run `cat << 'EOF' > benches/oil_paint_bench.rs` with the benchmark code for `apply_oil_paint`. Verify creation with `cat benches/oil_paint_bench.rs`.
2. **Update Cargo.toml:** Run `cat << 'EOF' > modify.py` with a python script to append the bench and example lines to `Cargo.toml`. Run `python3 modify.py`. Verify with `tail -n 20 Cargo.toml`.
3. **Create Module (Red Phase):** Run `cat << 'EOF' > crates/abrash-render/src/experimental/oil_paint.rs` containing `pub fn apply_oil_paint` and `mod tests`. Verify creation with `cat crates/abrash-render/src/experimental/oil_paint.rs`.
4. **Update Mod:** Run `echo "pub mod oil_paint;" >> crates/abrash-render/src/experimental/mod.rs`. Verify with `tail crates/abrash-render/src/experimental/mod.rs`.
5. **Run Red Phase:** Run `cargo test -p abrash-render --features nova -- oil_paint` to confirm the test fails.
6. **Implement Module (Green Phase):** Run `cat << 'EOF' > crates/abrash-render/src/experimental/oil_paint.rs` with the implementation for `apply_oil_paint`. Verify with `cat crates/abrash-render/src/experimental/oil_paint.rs`.
7. **Run Green Phase:** Run `cargo test -p abrash-render --features nova -- oil_paint` to confirm the test passes.
8. **Optimize Module (Refactor Phase):** Run `cat << 'EOF' > crates/abrash-render/src/experimental/oil_paint.rs` to include the `rayon` parallel chunks optimization. Verify with `cat crates/abrash-render/src/experimental/oil_paint.rs`.
9. **Run Benchmarks:** Run `cargo bench --bench oil_paint_bench --features nova`.
10. **Create Demo:** Run `cat << 'EOF' > examples/oil_paint_demo.rs` with a procedural pattern example that applies the filter, without assuming the existence of `Mesh`, `Vertex`, `Mat4`, `Vec3`, `Camera`, or `Rasterizer`. Verify creation with `cat examples/oil_paint_demo.rs`.
11. **Update Log:** Run `cat << 'EOF' >> .jules/nova.md` with the Concept, Fate, and Lesson.
12. **Run Linters and Tests:** Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all`, and `cargo test --all-features`.
13. **Pre-commit:** Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
14. **Submit:** Call `submit` tool with title "🌟 Nova: Oil Paint Filter" and the description format.
