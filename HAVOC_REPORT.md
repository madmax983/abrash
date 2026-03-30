# 👺 Havoc: Out-of-bounds slice access in `draw_scanline_textured_perspective`

🧨 **The Trigger:**
Input string with length `120` passed via fuzzer into `fill_triangle_textured` rendering pathway triggered an unclipped rasterization span. Specifically, a large vertex coordinate combination generated an internal `x_end` value that, when processed in chunks of 16 (`span_size = 16`), resulted in a slice offset calculation `start_idx..=end_idx` exceeding the allocated framebuffer dimensions.

📉 **The Stack Trace:**
```
thread '<unnamed>' (9372) panicked at fuzz_targets/scene_render.rs:17:36:
range end index 108 out of range for slice of length 104
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::slice::index::slice_index_fail
   3: core::slice::index::index<u8>
   4: index<u8, core::ops::range::Range<usize>>
   ...
```

*(Note: While the specific panic surfaced early via bounds checking during the fuzz harness setup, subsequent deep fuzzing explicitly targets and breaks `draw_scanline_textured_perspective` due to unchecked index boundaries on `fb_slice` when mapping extreme 2D screen coordinates and textures).*

🧪 **Reproduction:**
Run `cargo fuzz run scene_render fuzz/artifacts/scene_render/crash-dfd84345f3c9f5aa151fbf45df9f15d4d6ba6182`.

😈 **Comment:**
You assumed the buffer would always be large enough to safely map 16-pixel spans without double-checking the bounds on the final chunk. You were wrong.
