1. **Explore & Identify**
    - [x] Run benchmarks to identify performance bottlenecks in `clear_rect` operations.
    - [x] Analyze `Framebuffer::clear_rect` and `ZBuffer::clear_rect` in `crates/abrash-core/src/`.
2. **Implement Fixes**
    - [x] Optimize `Framebuffer::clear_rect` by using `.chunks_exact_mut()` with `get_unchecked_mut()` on rows.
    - [x] Optimize `ZBuffer::clear_rect` by using `.chunks_exact_mut()` with `get_unchecked_mut()` on rows for single-threaded path.
    - [x] Maintain safety by ensuring outer bounds `sx` and `ex` are securely clamped.
    - [x] Log learnings to `.jules/bolt.md` as per "Bolt" persona guidelines.
3. **Verification**
    - [x] Run `cargo bench --bench clear_rect_bench` to verify performance improvements.
    - [x] Run `cargo test` to ensure no functionality is broken by the optimizations.
    - [x] Ensure `cargo clippy` is clean.
4. **Pre Commit & Submit**
    - [x] Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
    - [ ] Submit PR using `submit` tool.
