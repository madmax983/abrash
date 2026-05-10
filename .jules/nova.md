## [Pointillism Filter]
**Concept:** A post-processing effect that simulates pointillism by iterating over a pixel grid with a specific step size and rendering filled circles (dots) colored based on the underlying source pixel, clearing the remaining background to a set color.
**Fate:** Implemented
**Lesson:** Replacing floating-point distance comparisons `((dx * dx + dy * dy) as f32 <= radius_sq)` with pre-scaled integer arithmetic `(((dx * dx + dy * dy) * 4) as u32 <= (dot_size * dot_size))` inside tight inner drawing loops provides significant performance improvements. Also, utilizing `thread_local!` buffers to store the source frame helps avoid per-frame heap allocations when reading and writing to the same framebuffer simultaneously.

## [Pointillism Filter]
**Concept:** A post-processing effect that simulates pointillism by iterating over a pixel grid with a specific step size and rendering filled circles (dots) colored based on the underlying source pixel, clearing the remaining background to a set color.
**Fate:** Implemented
**Lesson:** Replacing floating-point distance comparisons `((dx * dx + dy * dy) as f32 <= radius_sq)` with pre-scaled integer arithmetic `(((dx * dx + dy * dy) * 4) as u32 <= (dot_size * dot_size))` inside tight inner drawing loops provides significant performance improvements. Also, utilizing `thread_local!` buffers to store the source frame helps avoid per-frame heap allocations when reading and writing to the same framebuffer simultaneously.

## [Pointillism Filter]
**Concept:** A post-processing effect that simulates pointillism by iterating over a pixel grid with a specific step size and rendering filled circles (dots) colored based on the underlying source pixel, clearing the remaining background to a set color.
**Fate:** Implemented
**Lesson:** Replacing floating-point distance comparisons `((dx * dx + dy * dy) as f32 <= radius_sq)` with pre-scaled integer arithmetic `(((dx * dx + dy * dy) * 4) as u32 <= (dot_size * dot_size))` inside tight inner drawing loops provides significant performance improvements. Also, utilizing `thread_local!` buffers to store the source frame helps avoid per-frame heap allocations when reading and writing to the same framebuffer simultaneously.
