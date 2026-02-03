# 👺 Havoc Report: "The Abyss Gazes Back"

## 1. Denial of Service via Huge Triangles
**The Trigger:** A triangle with coordinates spanning from `-1e30` to `1e30`.

**The Mechanism:**
- The rasterizer loop iterates from `y_min` to `y_max`.
- `f32` to `i32` cast saturates to `i32::MIN` (-2B) and `i32::MAX` (2B).
- Result: 4,294,967,296 iterations per triangle.
- **Outcome:** Application hangs for >2 minutes per frame. DoS.

**The Fix:** Clamped the rasterization loop to the framebuffer height (`0..fb.height()`).

## 2. Panic via Allocation Overflow
**The Trigger:** `Framebuffer::new(u32::MAX, u32::MAX)`.

**The Mechanism:**
- `checked_mul` detects the overflow correctly.
- But `.expect("Buffer size overflow")` panics the thread.
- **Outcome:** Server/Application crash.

**The Fix:** Changed constructors to return `Result<Self, Error>` and propagate errors up the stack.

## 3. Undefined Behavior via NaN
**The Trigger:** Triangle with `NaN` coordinates.

**The Mechanism:**
- `NaN as i32` is technically Undefined Behavior in Rust (though saturates to 0 in practice on many targets).
- Comparisons with `NaN` yield `false`, breaking sorting logic (`v0`, `v1`, `v2` remain unsorted).
- **Outcome:** Unpredictable rendering, potential future UB.

**The Fix:** Added explicit `.is_finite()` checks to reject invalid geometry early.
