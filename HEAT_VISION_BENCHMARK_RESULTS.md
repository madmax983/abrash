# Heat Vision Benchmark Results

Before optimization:
Heat Vision/320x240     time:   [300.74 µs 302.98 µs 305.90 µs]
Heat Vision/800x600     time:   [1.9619 ms 1.9760 ms 1.9917 ms]
Heat Vision/1920x1080   time:   [8.9655 ms 8.9835 ms 9.0015 ms]

After SIMD Gather Optimization (Replacing LUT gather with pure SIMD Arithmetic):
Heat Vision/320x240     time:   [213.50 µs 213.66 µs 213.83 µs] (-28.0%)
Heat Vision/800x600     time:   [1.4113 ms 1.4136 ms 1.4161 ms] (-27.3%)
Heat Vision/1920x1080   time:   [6.4223 ms 6.4395 ms 6.4567 ms] (-26.2%)

Note: Replacing the LUT generation and `_mm256_i32gather_epi32` lookup with explicit bitwise SIMD logic (`sub`, `min`, `max`) completely bypassed the memory fetch bottleneck in the inner pixel loop, yielding a massive 26-28% overall performance improvement. Float math was not the actual bottleneck; memory bandwidth and SIMD gather instructions were.
