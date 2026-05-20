1. **Fix integer overflow in `AsciiConverter::to_colored_string`**
   - The method computes the capacity using `((width * 20) * height) as usize`. Since `width` and `height` are `u32`, this calculation will panic on `u32` overflow if `width * 20` exceeds `u32::MAX`, even if the actual number of pixels `width * height` is small (e.g., when `height` is 0).
   - Change it to use `usize` arithmetic: `let capacity = (width as usize).saturating_mul(20).saturating_mul(height as usize);`
   - Use `String::with_capacity(capacity)`
   - Verify by running `cargo test --test havoc_ascii_proptest` and expecting it to pass without panic.

2. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

3. **Submit the changes**
   - Commit and submit.
