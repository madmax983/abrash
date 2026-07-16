1. **Understand Request**: Create a Spirograph/Harmonograph module in the Rust Graphics Engine. "Spirograph" usually involves drawing roulette curves like hypotrochoids and epitrochoids.
2. **Review Memory/Persona**: The memory dictates acting as 'Nova' 🌟 to create a new, interesting feature from scratch in an isolated module `src/experimental/`.
3. **Plan Spirograph Implementation**:
   - `crates/abrash-render/src/experimental/spirograph.rs`: Implement a Spirograph struct with parameters (inner radius `r`, outer radius `R`, pen offset `d`), a method to generate points on the curve (hypotrochoid/epitrochoid), and a method to draw it onto a `Framebuffer`.
   - Update `crates/abrash-render/src/experimental/mod.rs` to expose `spirograph`.
   - Create an example `examples/spirograph_demo.rs` to showcase the feature.
   - Create a benchmark `benches/spirograph_bench.rs` and add it to `Cargo.toml`.
   - Write tests in `spirograph.rs`.
   - Run `cargo clippy`, `cargo fmt`, `cargo test`.
   - Log to `.jules/nova.md`.
4. **Follow TDD (Red/Green/Refactor)**:
   - Create tests first.
   - Implement the feature to pass tests.
   - Benchmark and optimize.
