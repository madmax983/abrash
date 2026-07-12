sed -i 's/format!("Scalar {}x{}", w, h)/format!("Scalar {w}x{h}")/g' benches/find_min_max_simd_bench.rs
sed -i 's/format!("SIMD {}x{}", w, h)/format!("SIMD {w}x{h}")/g' benches/find_min_max_simd_bench.rs
