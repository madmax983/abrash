# 👺 Havoc: Scanline ZBuffer Out-of-Bounds Panic

🧨 **The Trigger:**
Calling `draw_scanline_flat` (or any public scanline rasterization function that delegates to `prepare_scanline` without prior bounds matching) with mismatched `Framebuffer` and `ZBuffer` dimensions. Specifically, passing a large `Framebuffer` and a smaller `ZBuffer`.

📉 **The Stack Trace:**
```
thread 'havoc_test_draw_scanline_flat_zbuffer_oob_proptest' panicked at crates/abrash-render/src/rasterizer/core.rs:86:42:
range start index 3765 out of range for slice of length 209
```

🧪 **Reproduction:**
Run `cargo test -p abrash-render --test havoc_scanline -- --ignored`

😈 **Comment:**
"You exposed `draw_scanline_flat` as a public API but forgot to call `assert_same_dimensions(fb, zb)`. Then `prepare_scanline` blindly uses `fb.width()` to index into `zb`, causing a spectacular out-of-bounds panic. You assumed users would only use the high-level `fill_triangle_3d` pipeline. You were wrong. I bypass your safety guards and crash your renderer directly. Chaos wins again."
