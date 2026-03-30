## [Reduction]
**Bloat:** `let mut end = *other; if ... { end = ... }` in `Quat::nlerp` (`crates/abrash-core/src/quat.rs`) and strict float comparisons (`assert_eq!`) in `tests/math_extensions.rs`.
**Cut:** Replaced the useless `let mut` with an idiomatic `if`/`else` assignment expression. Changed the strict float equality assertions to evaluate against an `f32::EPSILON` boundary difference.
**Saved:** 1 warning, 4 warnings.
