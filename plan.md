1. **Explore and Identify**
   - Searched the codebase for occurrences of "trait", finding some leftovers in documentation and tests for `Lerpable`, `BspTextures`, `BspMap`, and `Evaluable` which were refactored to concrete structs or enums in previous "Razor" tasks.
2. **Execute Cleanup**
   - Cleaned up obsolete documentation references in `crates/abrash-core/src/clipping.rs` and `crates/abrash-core/src/curve.rs`.
   - Updated wording in `crates/abrash-anim/src/evaluable.rs` and `crates/abrash-raycast/src/renderer/bsp_lighting.rs` to refer to enum and concrete implementations rather than traits.
   - Removed `clippy::default_trait_access` attribute and used explicit default functions in `crates/abrash-gpu-render/src/lib.rs` and `crates/abrash-gpu-render/src/device.rs`.
   - Renamed test methods from `*_trait` to `*_conversion` in `crates/abrash-core/src/fixed16_16.rs`.
3. **Log Learnings**
   - Appended a log to `.jules/razor.md` documenting the cleanup of obsolete references to "trait".
4. **Complete pre commit steps**
   - Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
5. **Submit the change**
   - Submit the change with a descriptive commit message.
