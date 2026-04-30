# 👺 Havoc: [Fix Integer Overflows in Rect Rasterizer Bounds Math]

**🧨 The Trigger:**
Passing extreme positive and negative integers representing off-screen dimensions/coordinates (e.g., `x = 0`, `y = -1393964200`, `w = 1`, `h = 2147483648`) to `draw_rect` or `fill_rounded_rect` causes integer overflows/underflows in coordinate tracking math before checking screen bounds.

**📉 The Stack Trace:**
```
thread 'test_draw_rect_extreme_no_panic' panicked at crates/abrash-render/src/rasterizer/rect.rs:146:18:
attempt to add with overflow
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::panicking::panic_const::panic_const_add_overflow
   3: abrash_render::rasterizer::rect::draw_rect
             at ./src/rasterizer/rect.rs:146:18
   4: havoc_rect_proptest::test_draw_rect_extreme_no_panic::{{closure}}
             at ./tests/havoc_rect_proptest.rs:9:13
```
```
thread 'test_fill_rounded_rect_extreme_no_panic' panicked at crates/abrash-render/src/rasterizer/rect.rs:401:18:
attempt to multiply with overflow
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::panicking::panic_const::panic_const_mul_overflow
   3: abrash_render::rasterizer::rect::fill_rounded_rect
             at ./src/rasterizer/rect.rs:401:18
   4: havoc_rect_proptest::test_fill_rounded_rect_extreme_no_panic::{{closure}}
             at ./tests/havoc_rect_proptest.rs:16:13
```

**🧪 Reproduction:**
Run `cargo test -p abrash-render --test havoc_rect_proptest` to trigger property tests with maximum boundaries.

**😈 Comment:**
"You assumed dimensions and offscreen coordinates could never be larger than normal screen sizes before bounds checking. Math disagrees. Proptest found the exact overflow."
