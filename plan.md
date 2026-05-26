1. **Optimize `prepare_draws` allocation logic**
   - Write and run a Python script to update the signature and return logic of `prepare_draws` in `crates/abrash-gpu-render/src/renderer.rs` from `fn prepare_draws(&self, frame: &Frame) -> Result<(Vec<PreparedDraw>, Vec<u8>, u32), String>` to `fn prepare_draws(&self, frame: &Frame, prepared_draws: &mut Vec<PreparedDraw>, draw_bytes: &mut Vec<u8>) -> Result<u32, String>`.
   - Update the internal logic to mutate these buffers directly instead of creating them dynamically per call.

2. **Inject thread-local buffers into callers**
   - Write and run a Python script to update `capture()` and `render_to_surface()` in `crates/abrash-gpu-render/src/renderer.rs`.
   - Inject `thread_local! { static PREPARED_DRAWS: std::cell::RefCell<Vec<PreparedDraw>> = const { std::cell::RefCell::new(Vec::new()) }; static DRAW_BYTES: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) }; }` into these callers.
   - Update their calls to `prepare_draws` to pass the borrowed mutable references, eliminating dynamic heap allocations across frames.

3. **Verify performance improvement**
   - Use a Python script to create the benchmark file `crates/abrash-gpu-render/benches/gpu_prepare_draws.rs` to measure `prepare_draws`.
   - Add it to `Cargo.toml`.
   - Run `cargo bench -p abrash-gpu-render` to explicitly prove and document the performance win from eliminating these allocations.
   - Use `echo` to append the learning to `.jules/bolt.md`.

4. **Verify correct functionality**
   - Run `cargo check` and `cargo test -p abrash-gpu-render` to ensure the changes compile and run without runtime failures or regressions.

5. **Complete pre commit steps**
   - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
