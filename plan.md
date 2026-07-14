1.  **Red Phase (Create Tests & Scaffold):**
    *   Create `crates/abrash-render/src/experimental/datamosh.rs`.
    *   Define `DatamoshConfig` struct.
    *   Define `apply_datamosh` function.
    *   Write a unit test in `datamosh.rs` that verifies motion vectors (gradients) correctly shift pixels from the previous frame.
2.  **Green Phase (Implementation):**
    *   Use `thread_local! { static I_FRAME: RefCell<Vec<u32>> = ... }` to store the persistent I-Frame state.
    *   Calculate spatial luminance gradients of the current framebuffer (`dx` and `dy`).
    *   Use these gradients to sample the `I_FRAME` at offset `(x - dx, y - dy)`.
    *   Write the sampled pixel back to both the `I_FRAME` and the current framebuffer.
    *   Add an `i_frame_refresh` property to the config to occasionally copy the real frame to `I_FRAME`, resetting the moshing.
3.  **Refactor Phase (DRY/YAGNI):**
    *   Ensure the code uses `rayon` for parallel processing if the `parallel` feature is enabled (or maybe thread locals make parallelization tricky? Actually, with thread locals, we usually copy the frame to a local buffer, then process row-by-row. Wait, `I_FRAME` needs to be read from AND written to. Read from `I_FRAME`, write to a temporary, then swap. I will maintain a `PREV_FRAME` and write to the actual FB, then copy the FB to `PREV_FRAME`).
4.  **Integration:**
    *   Add `pub mod datamosh;` to `crates/abrash-render/src/experimental/mod.rs`.
    *   Create `examples/datamosh_demo.rs` to showcase the feature.
    *   Update `Cargo.toml` to include the new demo.
5.  **Pre-commit:**
    *   Run `pre_commit_instructions`.
    *   Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
    *   Run `cargo clippy`, `cargo test`, and `cargo fmt`.
6.  **Presentation (Submit):**
    *   Log learning to `.jules/nova.md`.
    *   Create the PR with the required Nova format.
