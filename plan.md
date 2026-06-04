1.  **Refactor Heat Vision to Fixed Point Math**
    - The current `heat_vision.rs` uses floating-point math (`(normalized - 0.25) * 4.0`, etc.) inside a hot pixel loop to map depths to colors.
    - We will map `0.0..1.0` depth range into integer space implicitly via the `get_unchecked(t as usize)` pattern by eliding bounds checks natively in the unrolled batch chunk loop using `min()`.
    - We will pre-calculate `min_z` and `max_z` like before, calculate a `z_range` integer mapping multiplier, and use simple `(depth - min_z) * scale` to avoid floats in the main loop while extracting slice variables outside the inner scalar loop and chunk loops to avoid out of bounds safety overhead.
2.  **Ensure Correctness**
    - Ensure all existing tests in `heat_vision.rs` pass.
3.  **Run Benchmark**
    - Ensure `cargo bench --bench heat_vision_bench` runs successfully and shows performance improvements.
4.  **Complete pre commit steps**
    - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5.  **Submit PR**
