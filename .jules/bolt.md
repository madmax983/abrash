**Heat Vision Range Finding**
**Learning:** Optimizing the min/max finding step of a two-pass algorithm (like depth auto-ranging) with AVX2 SIMD reduces the overhead significantly, leaving only the memory-bound second pass as the main bottleneck. Extracting the pass into a standalone function makes it easier to benchmark isolated performance improvements.
**Action:** Created `find_depth_range` internally delegating to `find_min_max_depth_simd` and wrote dedicated `Range_Only` Criterion benchmarks.
