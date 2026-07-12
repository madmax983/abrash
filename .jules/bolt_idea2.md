Optimization Idea 2:
Look at `draw_scanline_flat_simd` and `draw_scanline_flat_blended_simd`.
The scalar tail uses `zb_slice[i..len].iter_mut().zip(fb_slice[i..len].iter_mut())` which incurs bounds checking overhead inside a very hot loop. We can use `unsafe { zb_slice.get_unchecked_mut(...) }` to optimize it, as noted in the prompt memory:
"Persona 'Bolt' Learning: In extremely hot loops processing arrays within mathematically guaranteed bounds (e.g., scanline rasterization tails or image downsampling), safe slice indexing (like buffer[i..len].iter_mut()) incurs redundant bounds check overhead. Replacing these with unsafe { buffer.get_unchecked_mut(...) } safely bypasses this overhead, yielding measurable performance improvements."
