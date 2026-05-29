1. Add new fields `prepared_draws: Vec<PreparedDraw>` and `draw_bytes: Vec<u8>` to `GpuRenderer` in `crates/abrash-gpu-render/src/renderer.rs` to avoid dynamic allocations per frame in `prepare_draws`.
   - Update `GpuRenderer` struct definition to include these fields.
   - Initialize them in `from_gpu` with `Vec::with_capacity(128)` and `vec![]`.
   - Change `prepare_draws` signature to take `&mut self` and return just `u32` (total triangles) instead of a tuple.
   - Update the body of `prepare_draws` to `.clear()` and reuse `self.prepared_draws` and `self.draw_bytes`.
2. Update callers of `prepare_draws` (`capture` and `render_to_surface`) to use the new signature and then pass `&self.prepared_draws` or `&self.draw_bytes` where needed.
   - `capture` and `render_to_surface` both take `&mut self`.
   - We must call `let total_triangles = self.prepare_draws(frame)?;` first.
   - Then use `&self.prepared_draws` for `encode_shadow_pass`, `encode_gbuffer_pass`, `encode_rt_shadow_pass`, `encode_refraction_surface_pass`.
   - Wait, `encode_shadow_pass` requires `&mut self`. We cannot borrow `self.prepared_draws` and call a `&mut self` method at the same time.
   - To fix this borrow conflict, we can either extract `prepared_draws` using `std::mem::take(&mut self.prepared_draws)` before calling the passes, pass it in, and then restore it afterwards!
3. Implement `std::mem::take` pattern in `capture` and `render_to_surface`:
   - `let prepared_draws = std::mem::take(&mut self.prepared_draws);`
   - `let draw_bytes = std::mem::take(&mut self.draw_bytes);`
   - Use them for rendering.
   - At the end of the method, restore them: `self.prepared_draws = prepared_draws; self.draw_bytes = draw_bytes;`
4. Run tests and benchmarks: `cargo test -p abrash-gpu-render` and `cargo bench` to verify correctness.
5. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
