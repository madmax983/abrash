1.  **Refactor Sobel Filter**
    - In `crates/abrash-render/src/post_process/filters.rs`, modify the scalar `apply_sobel` fallback loop to use `.chunks_exact_mut()` instead of index math.
    - Keep `SOBEL_BUFFER` the way it is, but replace the nested loops iterating over `y` and calculating index offsets `row_offset`, `prev_row_offset`, `next_row_offset` with the pattern tested in `test_sobel.rs` (using `lum_slice.chunks_exact(width)` mapped via `zip(dest_rows.chunks_exact_mut(width))`).
2.  **Ensure Correctness**
    - Run `cargo test -p abrash-render --features "nova parallel"` to verify the changes didn't break functionality.
3.  **Run Benchmark**
    - Run `cargo bench -p abrash-render --bench filters_bench --features "nova parallel"` to verify performance improvements.
4.  **Complete pre commit steps**
    - Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
5.  **Submit PR**
