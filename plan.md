1. **Optimize ZBuffer min/max scan**
   - The current code uses a scalar loop with a branch (`if z != f32::INFINITY`) for every pixel to find `min_z` and `max_z`. We can optimize this by using SIMD for the min/max search, just like how `apply_heat_vision_simd` handles the color conversion. We can extract min/max computation into a fast SIMD block.
2. **Optimize Framebuffer conversion scalar tail**
   - The scalar `for` loops in both `apply_heat_vision` and the tail of `apply_heat_vision_simd` can be slightly optimized by using chunks or `unroll` if manual indices perform better.
3. **Verify tests and benchmarks**
   - Ensure `heat_vision` tests and benchmarks show improvement without regression.
