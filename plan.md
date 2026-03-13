1. **Audit `src/experimental/directional_blur.rs`**
   - The `apply_directional_blur` method in `src/experimental/directional_blur.rs` needs extensive testing.
   - It is missing tests for vertical blur, combined dx/dy blur, edge cases like extremely large sample counts, or blurring when width/height is 0.

2. **Implement Tests in `src/experimental/directional_blur.rs`**
   - Write comprehensive tests for `test_directional_blur_vertical`.
   - Write tests for `test_directional_blur_diagonal`.
   - Write tests for `test_directional_blur_zero_size_framebuffer`.
   - Write tests for `test_directional_blur_huge_samples`.

3. **Verify**
   - Run `cargo test --no-default-features --features "backend-tui parallel nova" -- directional_blur` to make sure the newly added tests compile and pass.
   - Run `cargo clippy --all-targets --no-default-features --features "backend-tui parallel nova" -- -D warnings`.
   - Make sure no regressions are introduced.

4. **Pre-commit Steps**
   - Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
