# Razor's Journal

## [Reduction]
**Bloat:** Deprecated `Vec3::zero` and `Mat4::mul` methods that were just wrappers or old implementations.
**Cut:** Removed methods and moved `Mat4` multiplication logic directly into `Mul` trait implementation.
**Saved:** 20 lines of code and reduced API surface area.

## [Reduction]
**Bloat:** Unused `fill_triangle_flat` wrapper function that duplicated `fill_triangle_lit` logic with hardcoded lights.
**Cut:** Removed `fill_triangle_flat` and updated tests to use `fill_triangle_lit`.
**Saved:** ~30 lines of code + test cleanup.

## [DRY]
**Bloat:** Duplicated `Event` and `WindowError` enums in `win32.rs` and `dummy.rs`.
**Cut:** Moved common types to `src/platform/mod.rs` and shared them.
**Saved:** ~30 lines of duplication.
