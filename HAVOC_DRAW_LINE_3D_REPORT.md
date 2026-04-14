# 👺 Havoc: Line Rasterization ZBuffer Out-of-Bounds Write

🧨 **The Trigger:**
Passing a `Framebuffer` and `ZBuffer` with mismatched dimensions to the `draw_line_3d` rasterization function. Specifically, passing a large `Framebuffer` (e.g., 100x100) and a smaller `ZBuffer` (e.g., 10x10).

📉 **The Stack Trace:**
```
thread 'test_draw_line_3d_zbuffer_oob' panicked at crates/abrash-render/src/rasterizer/line.rs:74:58:
unsafe precondition(s) violated: slice::get_unchecked_mut requires that the index is within the slice

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
thread caused non-unwinding panic. aborting.
```

🧪 **Reproduction:**
Run `cargo test --test havoc_draw_line_3d -- --ignored`
This executes the ignored proptest which intentionally triggers the SIGABRT crash. The test must be ignored by default so it doesn't crash the cargo test runner during standard suite checks.

😈 **Comment:**
"You checked bounds against the Framebuffer and assumed the ZBuffer was identical in size. You bypassed standard bounds checks with `unsafe { get_unchecked_mut }` but failed to `assert_same_dimensions(fb, zb)` like your other rasterizers. You left the door open to arbitrary memory corruption. Chaos wins again."
