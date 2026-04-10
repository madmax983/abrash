## [Extract math blob into cohesive submodules]
**Tangle:** The `crates/abrash-core/src/math.rs` file grew into a massive 20,000-line "Blob" anti-pattern, violating structural cohesion by bundling unrelated matrix, vector, quaternion, and utility functions into a single namespace. Furthermore, it duplicated the `Quat` struct which already existed in its own `quat.rs` module, creating ambiguous boundaries.

**Blueprint:**
1. **Extract Types:** Split `Vec2`, `Mat2`, `Vec3`, `Mat4`, `ScreenPoint`, `Vec4`, and `Mat3` into their own focused files inside a new `crates/abrash-core/src/math/` directory.
2. **Remove Redundancy:** Deleted the duplicated `Quat` implementation from `math.rs` and aligned its unit tests to use the standard `crates/abrash-core/src/quat.rs` implementation (e.g. converting `.rotate()` to `.rotate_vec3()`).
3. **Facade API:** Created `crates/abrash-core/src/math/mod.rs` to re-export the submodules `pub use vec3::*;`, ensuring zero breaking changes to the external public API.
4. **Utilities:** Moved the remaining mathematical utility functions to `crates/abrash-core/src/math/utils.rs`.

**Stability:** High cohesion across fundamental linear algebra types. Drastically reduced file sizes, making the core domain much easier to read and maintain. Reduced coupling by relying on a single source of truth for quaternions.

**Verification:** Validated that the entire `abrash-core` crate complies and passes all tests with the restructured domain boundaries.
