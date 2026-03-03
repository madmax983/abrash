# 👺 Havoc: PreparedTriangle i16 Integer Wrap Vulnerability

🧨 **The Trigger:**
Rendering a triangle with screen coordinates exceeding `32767` (e.g., `X = 65586.0`) causes silent, catastrophic integer wrap-around.

📉 **The Stack Trace (Conceptual):**
When converting from floating-point projected space to `CompactScreenPoint`, coordinates are cast down to `i16`:
```rust
pub struct CompactScreenPoint {
    pub x: i16,
    pub y: i16,
    pub z: f32,
}
```
If `X = 65586.0`, it exceeds `i16::MAX` (`32767`). The cast wraps it around: `65586 - 65536 = 50`.
The tile binning system then calculates `aabb_min_x` and `aabb_max_x` based on these corrupted coordinates, placing the huge triangle into tile `X=0, Y=0`.
Instead of properly clipping the off-screen triangle out of existence, it is rendered precisely in the center of the screen as a ghost artifact.

🧪 **Reproduction:**
Run the newly created exploit test:
```bash
cargo test --no-default-features --features "backend-tui nova parallel" test_havoc_prepared_triangle_i16_overflow
```
Output:
```
thread 'test_havoc_prepared_triangle_i16_overflow' panicked at tests/havoc_i16_overflow.rs:48:5:
Triangle with X=65586 wrapped around and rendered on screen!
```

😈 **Comment:**
"You assumed no one would ever render a zoomed-in triangle larger than 32,767 pixels. You were wrong. 64K resolutions and extreme close-ups just shattered your reality."
