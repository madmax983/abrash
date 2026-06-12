**coverage_percent Edge Case in GpuDebugCapture**
**Learning:** `GpuDebugCapture::coverage_percent()` returns a division by zero error/NaN when there are no pixels (a zero-sized capture). Though there is a check if total_pixels == 0, it wasn't tested in Rust.
**Action:** Adding a test for `coverage_percent()` when `total_pixels` is zero to ensure that `f32::EPSILON` bounds correctly and division by zero is handled safely.

**[Validating Unreachable Guards]**
**Learning:** To verify internal `unreachable!()` or panic safety guards against corrupted state, write unit tests in the same module (`mod tests`) to access private fields, intentionally mutate the internal state to simulate the illegal condition, and assert the guard triggers using `#[should_panic(expected = "...")]`.
**Action:** Always write a corresponding `#[should_panic]` test for any defensive `unreachable!()` statements that check internal invariants, directly testing the explosion.

**[Validating Expect Guards on Allocation Dimensions]**
**Learning:** Calculations that multiply dimensions (like `width * height`) to determine allocation size can easily overflow `usize`, leading to panics via `.expect("... overflow")` or implicitly during allocation. These guards are critical for security and stability but are often untested because they require absurdly large inputs (like `u32::MAX`).
**Action:** Always write a corresponding `#[should_panic]` test for bounds checking logic covering allocation counts by testing the explicit limits (e.g., `u32::MAX`).

**[Total Ordering for Floats]**
**Learning:** Using `.partial_cmp().unwrap()` to sort floating-point numbers can panic when encountering `NaN` values, and `.unwrap_or(Ordering::Equal)` breaks the strict total ordering requirement of sorting algorithms. Using an unhandled `.unwrap()` at the end of a `min_by` or `max_by` call on iterators can also panic if the collection is empty.
**Action:** Use `f32::total_cmp` (or `f64::total_cmp`) instead of `partial_cmp` to provide a robust total ordering when sorting or finding min/max values in float slices. Also, replace terminal `.unwrap()` calls on iterators with `.unwrap_or()` or `.map_or()` to handle empty collections safely without panics.
