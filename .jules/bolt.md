Optimization Idea 3:
Reviewing `crates/abrash-core/src/rasterizer/flat.rs` `draw_scanline_flat_simd` again. The scalar tail loop and the main scalar loop can be heavily optimized by using unchecked indexing (which was our attempt 2 but it failed because `j` in `i..len` needs to be used safely, and also zip is slow in unoptimized builds but usually well-optimized by LLVM).

Actually, the previous fix to `plasma` procedural fixed the compiler warning. Let's create an execution plan for a different optimization or stick to the procedural fix. Wait, the user prompt is "Build a Rust Graphics Engine. Use Red Phase/Green Phase/Refactor TDD approach. Make benchmarks for your changes, and optimize for performance."

Let's do the TDD approach and add a new procedural texture or effect. Let's do a `pixelate` effect or `checkerboard`. No, I can fix the `clippy` warning first, and we can check what's missing. I fixed the clippy warning. Let's make an execution plan and submit.

Wait, the prompt says "Build a Rust Graphics Engine. Use Red Phase/Green Phase/Refactor TDD approach. Make benchmarks for your changes, and optimize for performance."
Let's see if there are any existing memory constraints.
`Persona 'Bolt' Learning: In extremely hot loops processing arrays within mathematically guaranteed bounds (e.g., scanline rasterization tails or image downsampling), safe slice indexing (like buffer[i..len].iter_mut()) incurs redundant bounds check overhead. Replacing these with unsafe { buffer.get_unchecked_mut(...) } safely bypasses this overhead, yielding measurable performance improvements.`

We tried it but it didn't yield a performance improvement in `flat.rs`, probably because `scanline_micro` bench tests a large triangle where the tail is small. What about `scanline_1920x1080`? Or `fill_cube_tiled`?

Let's do TDD to add a new effect `checkerboard` in `procedural.rs`.
