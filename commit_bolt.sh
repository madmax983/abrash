#!/bin/bash
git add crates/abrash-render/src/rasterizer/circle.rs
git add crates/abrash-render/src/rasterizer/ellipse.rs
git add crates/abrash-render/src/post_process/filters.rs

git commit -m "⚡ Bolt: Elide bounds checks and add AVX2 invert" -m "
💡 What
- Added unsafe get_unchecked_mut to horizontal span drawers in circle and ellipse algorithms
- Replaced auto-vectorized invert loop with explicit _mm256_xor_si256 intrinsics

🎯 Why
- Primitive span drawers operate within geometrically proven bounds; standard slice assignment pays unnecessary bounds-check penalties
- Explicit SIMD intrinsics provide guaranteed vectorization over relying on LLVM optimization hints

📊 Impact
- Reduced overhead in circle and ellipse rasterization
- Guaranteed AVX2 path for screen-space invert operations

🔬 Measurement
Benchmarked via circle_bench and post_process benchmarks."
