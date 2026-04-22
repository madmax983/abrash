## 👹 Havoc: SIMD Slice Length Vulnerabilities

### 🧨 The Trigger
In the SIMD rasterizers `draw_span_nearest_simd` (Texture) and `draw_scanline_gouraud_simd_fast` (Gouraud), the primary iteration loop uses the length of the framebuffer (`fb_slice.len()`) to determine iteration boundaries.

However, both the frame buffer (`fb_slice`) and the depth buffer (`zb_slice`) are passed independently. If a smaller depth buffer is passed, the SIMD implementation reads and writes beyond the boundaries of `zb_slice`, resulting in memory corruption and a hard crash (SIGSEGV).

### 📉 The Stack Trace
```
running 1 test
test rasterizer::texture_havoc_test::havoc_texture_simd_buffer_mismatch ... FAILED
thread 'rasterizer::texture_havoc_test::havoc_texture_simd_buffer_mismatch' (8878) panicked at crates/abrash-render/src/rasterizer/texture.rs:1148:38:
unsafe precondition(s) violated: slice::get_unchecked_mut requires that the index is within the slice

test rasterizer::gouraud_havoc_test::havoc_gouraud_simd_buffer_mismatch ... FAILED
thread 'rasterizer::gouraud_havoc_test::havoc_gouraud_simd_buffer_mismatch' panicked at crates/abrash-render/src/rasterizer/gouraud.rs:129:39:
```

### 🧪 Reproduction
Run `cargo test -p abrash-render -- --ignored havoc_texture_simd_buffer_mismatch` or `cargo test -p abrash-render -- --ignored havoc_gouraud_simd_buffer_mismatch`.

### 😈 Comment
You blindly assumed that your slices would always be identical lengths without enforcing it. I provided a mismatch, and the system segfaulted under the sheer weight of its own Hubris. I win. 👺
