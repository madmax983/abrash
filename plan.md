1.  **Refactor Heat Vision to Fixed Point Math**
    - The current `heat_vision.rs` uses floating-point math (`(normalized - 0.25) * 4.0`, etc.) inside a hot pixel loop to map depths to colors.
    - We will follow the `[Posterize Float to Integer Math Optimization]` learning from `.jules/bolt.md` to map `0.0..1.0` depth range into integer `0..255`, and calculate RGB entirely using integer math (`t * 255 / scale`, etc.).
    - We will pre-calculate `min_z` and `max_z` like before, calculate a `z_range` integer mapping multiplier, and use simple `(depth - min_z) * scale` to avoid floats in the main loop.
2.  **Ensure Correctness**
    - Ensure all existing tests in `heat_vision.rs` pass.
3.  **Run Benchmark**
    - Ensure `cargo bench --bench heat_vision_bench` runs successfully and shows performance improvements.
4.  **Complete pre commit steps**
    - Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
5.  **Submit PR**
