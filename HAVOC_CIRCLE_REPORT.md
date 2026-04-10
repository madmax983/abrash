# 👺 Havoc: Circle Rasterization Integer Overflow

🧨 **The Trigger:**
Providing an extremely large radius (e.g., `2_000_000_000`) to the circle rasterizer functions (`draw_circle` and `fill_circle`).

📉 **The Stack Trace:**
```
thread 'test_fill_circle_overflow' panicked at crates/abrash-render/src/rasterizer/circle.rs:171:21:
attempt to multiply with overflow
```

🧪 **Reproduction:**
Run `cargo test --test havoc_circle_overflow`
This executes `tests/havoc_circle_overflow.rs` which demonstrates the panic when bounds checks do not protect mathematical formulas.

😈 **Comment:**
"You assumed no one would ever want a circle larger than `i32::MAX / 2`. You assumed the math was safe from users throwing massive integers at your API. You were wrong. Chaos reigns."
