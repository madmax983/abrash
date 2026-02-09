# Hi-Z Benchmark Results

## Baseline (Before Optimization)
- `hiz_build_1080p`: ~4.5 ms
- `hiz_query_1000_aabbs`: ~21.5 µs

## After Optimization (Scalar Loop Unrolling)
- `hiz_build_1080p`: ~3.7 ms (-17%)
- `hiz_query_1000_aabbs`: ~21.9 µs (No change)

## Optimization Details
The `build_level_scalar` function was refactored to process the main 2x2 block area separately from boundary conditions. This allowed removing repeated bounds checks and `unwrap_or` calls in the inner loop, resulting in more efficient instruction generation and better cache utilization.
