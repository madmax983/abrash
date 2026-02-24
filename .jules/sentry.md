# Sentry's Journal

**[Math Library Normality]**
**Learning:** `Vec3::normalize` in this codebase is non-standard: it returns `self` (the original vector) if length is < 0.0001, instead of a zero vector or NaN.
**Action:** When testing math functions, account for this specific behavior to avoid false positives.

**[Pipeline Coordinates]**
**Learning:** Pipeline functions like `fill_triangle_flat` accept homogeneous Clip Space coordinates `(Vec3, w)`, which are then projected to Screen Space. Tests for off-screen culling must provide Clip Space coordinates that project to off-screen pixels.
**Action:** Calculate expected Screen Space coordinates manually when writing pipeline tests.

**[Integer Overflow in Backface Culling]**
**Learning:** `ScreenPoint` coordinates use `i32` but differences can be `~4e9`. Cross product `ux * vy` can exceed `i64::MAX`, causing panic in debug builds.
**Action:** Use `i128` for intermediate cross product calculations when working with full-range `i32` screen coordinates.

**[SIMD vs Scalar Inconsistency]**
**Learning:** `_mm_cvttps_epi32` behaves differently than scalar `as i32` for out-of-range floats (wraps to `i32::MIN` vs saturates to `i32::MAX`). This caused massive visual glitches for large coordinates.
**Action:** Always clamp float inputs to `[i32::MIN as f32, i32::MAX as f32]` before converting to integer in SIMD paths.

**[SIMD Newton-Raphson & Infinity]**
**Learning:** `_mm_rcp_ps` returns 0 for `Inf`, but the Newton-Raphson refinement `y0 * (2 - x * y0)` produces `0 * (2 - Inf * 0)` -> `0 * (2 - NaN)` -> `NaN` when `x` is `Inf`.
**Action:** Clamp inputs to a large finite value (e.g., `1e30`) before `_mm_rcp_ps` if `Inf` is a possible input and exact precision isn't required for large values.
