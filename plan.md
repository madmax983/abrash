1. **Threat Analysis:**
   The `Texture` struct in `crates/abrash-core/src/texture.rs` has several fields marked as `pub`, notably `width`, `height`, `width_shift`, and `pixels`. Because these fields are public, safe code can mutate them independently. The safe method `get_pixel_texel()` relies on `width`, `height`, and `width_shift` to calculate memory offsets for `unsafe { *self.pixels.get_unchecked(...) }`. By mutating these public fields to invalid values (e.g. making `width` very large, or changing `width_shift`), an attacker can cause `get_pixel_texel` to read out-of-bounds memory, leading to Undefined Behavior.

2. **Defense Strategy:**
   - Modify the `Texture` struct to make its fields private (`pub(crate)` where necessary or fully private).
   - Provide safe accessor methods (`width()`, `height()`, `pixels()`, `pixels_mut()`, etc.) so that the rest of the application can still read the dimensions and mutate the pixel data safely, but cannot change the dimensions and `width_shift` independently of the underlying buffer's length.
   - Update all references to `Texture`'s fields throughout the codebase to use these safe accessors.
   - Add a test case demonstrating the safety properties or validating that `get_pixel_texel` works without exposing the vulnerability.

3. **Execution Steps:**
   - Step 1: Modify `crates/abrash-core/src/texture.rs` to encapsulate `Texture` fields.
   - Step 2: Implement getters (`width()`, `height()`, `pixels()`, `pixels_mut()`, `mips()`, `filter_mode()`, `set_filter_mode()`, `address_mode()`, `set_address_mode()`).
   - Step 3: Update `crates/abrash-render/` and `crates/abrash-core/` and other crates to use these new getters/setters instead of directly accessing `texture.width`, `texture.pixels`, etc.
   - Step 4: Ensure all tests pass.
   - Step 5: Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
   - Step 6: Submit the changes.
