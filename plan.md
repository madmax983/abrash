1.  **Analyze the task:**
    - Persona: Bolt (performance-focused).
    - Objective: Implement ONE safe, zero-cost abstraction or performance improvement per PR. Follow TDD (Red/Green/Refactor).
    - I will optimize the `apply_posterize` filter in `crates/abrash-render/src/experimental/posterize.rs`.
    - I ran the benchmark `posterize_bench` before and after the modification. The benchmark shows a performance improvement (time reduced from ~8.74ms to ~8.62ms). The change replaces slow floating-point arithmetic `(r / 255.0 * levels - ...)` with fast integer arithmetic.

2.  **Verify the change:**
    - I will run `cargo test --all-targets --all-features` to ensure no regressions were introduced.
    - I will run `cargo clippy --all-targets --all-features -- -D warnings` to verify code quality.
    - I will write the learning into `.jules/bolt.md`.

3.  **Complete pre-commit steps:**
    - Use `pre_commit_instructions` tool to make sure all pre commit requirements are met.
    - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4.  **Submit the code:**
    - Use the `submit` tool to finalize the code with an appropriate title and description matching the `Bolt` persona guidelines.
