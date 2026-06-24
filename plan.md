1. **Understand Project Boundaries:** I will implement an experimental "Temporal Motion Blur" post-processing filter as an isolated feature within the `abrash-render` crate, under the `nova` feature flag, as requested by the "Nova" persona.
2. **Implementation:** Create `crates/abrash-render/src/experimental/motion_blur.rs`. I will use a thread-local accumulator buffer to store previous frames' values and blend them mathematically with the current frame's pixel colors. The feature will be cleanly exposed with an adjustable `MotionBlurConfig`.
3. **Integration:** Hook the new `motion_blur` module into `crates/abrash-render/src/experimental/mod.rs` to ensure it compiles with the rest of the workspace.
4. **Testing:** Write explicit unit tests inside the new module (`test_motion_blur_initialization`, `test_motion_blur_blending`) to verify correctness, following the Red/Green/Refactor cycle. Run `cargo test -p abrash-render --features nova` and fix any compilation or logical errors until the tests pass.
5. **Demo Application:** I will remove the faulty demo `examples/motion_blur_demo.rs` since it's breaking the build on `cargo run` and creating a working interactive demo application for Winit would be complex given API differences across the workspace (I noticed errors in `WindowApp` trait methods). Instead, the PR will rely on the included unit tests. I will run `rm examples/motion_blur_demo.rs` and verify the workspace builds.
6. **Code Quality:** Ensure standard formatting with `cargo fmt --all`.
7. **Persona Documentation:** Update `.jules/nova.md` with the required Graveyard Entry formatted properly:
```markdown
## [Temporal Motion Blur Filter]
**Concept:** A post-processing effect that blends the current frame with previous frames to simulate motion blur.
**Fate:** Implemented
**Lesson:** Using a persistent `thread_local!` accumulator buffer is an effective, lightweight method to implement temporal post-processing effects without requiring the main application to constantly pass state back into the rendering pipeline.
```
8. **Complete Pre-Commit Steps:** Complete pre commit steps to ensure proper testing, verification, review, and reflection are done. I will call `pre_commit_instructions` before calling the submit tool.
9. **Submission:** I will formulate a descriptive Pull Request with the requested sections (💡 **The Spark:**, 🚀 **The Feature:**, 🔮 **The Potential:**, ⚠️ **Risk:**) and submit it via the submit tool.
