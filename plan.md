1. **Optimize floating-point infinity checks in `heat_vision.rs`**
   - Replace strict float `== f32::INFINITY` and `!= f32::INFINITY` equality checks with the integer bitwise equivalent `z.to_bits() == 0x7F80_0000` (or `!=`).
   - This bypasses the FPU entirely in the hot scalar rendering path, eliminating comparison overhead.
2. **Optimize floating-point infinity checks in `paper_cutout.rs`**
   - Apply the same optimization replacing `== f32::INFINITY` with `to_bits() == 0x7F80_0000` in the `apply_paper_cutout` loop.
3. **Verify impact**
   - Run `cargo test -p abrash-render --all-features` to ensure no functionality is broken.
   - Run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done**
   - Use the `pre_commit_instructions` tool to execute standard pre-submit checks.
5. **Submit the PR**
   - Commit and submit the code with a descriptive PR tracking the optimization details.
