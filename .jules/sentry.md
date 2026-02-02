# Sentry's Journal

**[Math Library Normality]**
**Learning:** `Vec3::normalize` in this codebase is non-standard: it returns `self` (the original vector) if length is < 0.0001, instead of a zero vector or NaN.
**Action:** When testing math functions, account for this specific behavior to avoid false positives.

**[Pipeline Coordinates]**
**Learning:** Pipeline functions like `fill_triangle_flat` accept homogeneous Clip Space coordinates `(Vec3, w)`, which are then projected to Screen Space. Tests for off-screen culling must provide Clip Space coordinates that project to off-screen pixels.
**Action:** Calculate expected Screen Space coordinates manually when writing pipeline tests.

**[Pipeline Coordinate Overflow]**
**Learning:** Screen space coordinates can overflow `i32` when `w` is very small, causing panics in difference calculations like `p2.x - p0.x` or `attempt to negate with overflow`.
**Action:** Cast screen coordinates to `i64` before subtraction and use proper parentheses `-(x as f32)` for negation.
