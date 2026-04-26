1. **Explore codebase and identify missing tests based on Sentry's journal:**
   - The user asked to act as "Sentry" and identify untested logic, edge cases, and potential panic points.
   - Sentry's journal mentions:
     `**[Validating Expect Guards on Allocation Dimensions]**`
     `**Learning:** Calculations that multiply dimensions (like width * height) to determine allocation size can easily overflow usize, leading to panics via .expect("... overflow") or implicitly during allocation.`
     `**Action:** Always write a corresponding #[should_panic] test for bounds checking logic covering allocation counts by testing the explicit limits (e.g., u32::MAX).`
   - In `crates/abrash-gpu-render/src/capture.rs`, there is a function `aligned_bytes_per_row` that has an `.expect("capture width overflowed RGBA byte count")` for `width.checked_mul(4)`.
   - The same guard is present in `CaptureConfig::new(width: u32, height: u32)`.
   - There was NO test covering this panic.

2. **Add the missing test `test_capture_target_dimensions_overflow` in `crates/abrash-gpu-render/src/capture.rs`:**
   - I will use `replace_with_git_merge_diff` to add the `test_capture_target_dimensions_overflow` test to `crates/abrash-gpu-render/src/capture.rs` within the `tests` module.
   - The test will look like:
     ```rust
     #[test]
     #[should_panic(expected = "capture width overflowed RGBA byte count")]
     fn test_capture_target_dimensions_overflow() {
         let _ = CaptureConfig::new(u32::MAX, 240);
     }
     ```

3. **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done:**
   - Run `cargo fmt --all`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo test -p abrash-gpu-render`.
   - Execute the `pre_commit_instructions` tool to perform the full pre-commit verification sequence.

4. **Submit the PR:**
   - Title: "🛡️ Sentry: [test coverage improvement]"
   - Description with target, risk, strategy, and verification.
