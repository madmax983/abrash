## [Reduction]
**Bloat:** `WindowBackend` trait with only two compile-time exclusive implementations (`Win32Window`, `TuiWindow`).
**Cut:** Removed `WindowBackend` trait and replaced with inherent method implementations on `Win32Window` and `TuiWindow`.
**Saved:** Reduced abstraction layer, simplified `examples/` imports.

## [Reduction]
**Bloat:** Unused `Mat2` struct and associated methods in `src/math.rs` and tests.
**Cut:** Deleted `Mat2` struct and its tests.
**Saved:** ~50 lines of code + cognitive load of maintaining unused math primitive.

## [Reduction]
**Bloat:** `src/experimental` module containing unused procedural generation code (`procedural.rs`, `terrain.rs`) and tests.
**Cut:** Deleted `src/experimental` directory and `tests/procedural_tests.rs`.
**Saved:** ~300 lines of dead code.
