## [Radar Sweep Filter]
**Concept:** A retro post-processing effect that simulates a classic radar screen. Features a rotating beam with a fading tail, distance rings, and a green luminescent tint over the original framebuffer.
**Fate:** Implemented
**Lesson:** Simple polar coordinate mathematics (`atan2` and distance) create a highly effective sweeping effect. Preserving the background luminance while applying a tinted overlay produces a convincing retro-tech feel without modifying core logic. Using `.min(1.0)` for overlay intensity keeps the math simple and bounded.

## [Radar Sweep Filter]
**Concept:** A retro post-processing effect that simulates a classic radar screen. Features a rotating beam with a fading tail, distance rings, and a green luminescent tint over the original framebuffer.
**Fate:** Implemented
**Lesson:** Simple polar coordinate mathematics (`atan2` and distance) create a highly effective sweeping effect. Preserving the background luminance while applying a tinted overlay produces a convincing retro-tech feel without modifying core logic. Using `.min(1.0)` for overlay intensity keeps the math simple and bounded.
