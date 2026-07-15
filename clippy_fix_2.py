with open("benches/find_min_max_simd_bench.rs", "r") as f:
    code = f.read()

code = code.replace('format!("Scalar {}x{}", w, h)', 'format!("Scalar {w}x{h}")')
code = code.replace('format!("SIMD {}x{}", w, h)', 'format!("SIMD {w}x{h}")')

with open("benches/find_min_max_simd_bench.rs", "w") as f:
    f.write(code)
