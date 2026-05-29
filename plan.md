1. **Optimize `transform_points_avx2` with FMA.**
   - In `crates/abrash-core/src/math/mat4.rs`, replace the nested `_mm256_add_ps` and `_mm256_mul_ps` sequence in `transform_points_avx2` with `_mm256_fmadd_ps` to reduce instruction count and latency, improving matrix transformation performance.
2. **Optimize `transform_points_scalar_uninit` and `transform_points_affine` loops.**
   - Reorder arithmetic to take advantage of compiler auto-vectorization and instruction pipelining, and investigate `fma` usage if appropriate for scalar floats.
3. **Verify performance improvements.**
   - Run `cargo bench --bench transform_points_bench` to ensure significant execution speed improvements.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Execute `cargo clippy`, `cargo test`, and `cargo fmt`. Update `clippy.log` and document learnings to `.jules/bolt.md`.
5. **Submit the PR.**
   - Title: `⚡ Bolt: Use FMA in transform_points AVX2`.
