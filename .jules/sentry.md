# Sentry's Journal

**[Math Library Normality]**
**Learning:** `Vec3::normalize` in this codebase is non-standard: it returns `self` (the original vector) if length is < 0.0001, instead of a zero vector or NaN.
**Action:** When testing math functions, account for this specific behavior to avoid false positives.

**[Pipeline Coordinates]**
**Learning:** Pipeline functions like `fill_triangle_flat` accept homogeneous Clip Space coordinates `(Vec3, w)`, which are then projected to Screen Space. Tests for off-screen culling must provide Clip Space coordinates that project to off-screen pixels.
**Action:** Calculate expected Screen Space coordinates manually when writing pipeline tests.

**[Integer Overflow in Rasterization]**
**Learning:** `fill_triangle_3d` calculated span width `dx` using `i32` subtraction (`x_end - x_start`). When a triangle spans from `i32::MIN` (clipped/projected) to positive coordinates, this overflows to negative, causing the span to be silently skipped.
**Action:** Always check intermediate arithmetic for coordinate differences, especially when screen coordinates can be saturated to `i32::MIN`/`i32::MAX`. Use `i64` for span calculations.
