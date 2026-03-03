# Sentry's Journal

## [Framebuffer `clear_rect` Out-of-Bounds]
**Learning:** The `clear_rect` function in `Framebuffer` panicked when arguments were fully out of bounds or negatively out of bounds due to incorrect scaling of the start/end slices, as `y.saturating_add` on large heights could be smaller than expected. Also, `clamp` was useful to enforce proper boundaries.
**Action:** When working with grid buffers and arbitrary rectangular boundaries, use `clamp(0, max_size)` for correct region intersection, ensuring `start <= end` before looping over the rows.
