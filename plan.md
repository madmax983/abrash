1. **Explore the codebase and understand the task**
   - Verified that the persona is "Nova", tasked with creating ONE new, interesting feature from scratch (additive only) without modifying core logic.
   - Decided to create a Physarum (Slime Mold) Simulation post-processing effect in `crates/abrash-render/src/experimental/physarum.rs`.

2. **Implement the new feature**
   - Created `crates/abrash-render/src/experimental/physarum.rs` with the `apply_physarum` function. It simulates slime mold agents depositing pheromones and moving based on the trail map.
   - Used thread-local `RefCell`s to manage state without reallocations (`TRAIL_MAP` and `AGENTS`).

3. **Wire it up**
   - Added `physarum` module to `crates/abrash-render/src/experimental/mod.rs`.
   - Created `examples/physarum_demo.rs` to demonstrate the effect.
   - Added `benches/physarum_bench.rs` to measure performance.
   - Updated `Cargo.toml` to register the new example and bench.

4. **Verify correctness**
   - Fixed compilation errors due to missing imports (`XorShiftRng`, `color_blend`).
   - Fixed borrow checker issue with `trail_borrow`.
   - Ensured no panics on `.unwrap()` calls for system time.
   - Validated that `cargo check`, `cargo test`, and `cargo clippy` pass cleanly.
   - Logged the idea in `.jules/nova.md`.

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done**
   - Use `pre_commit_instructions` tool to make sure all pre commit requirements are met.

6. **Submit the changes**
   - Use the `submit` tool to finalize.
