## [RetroFX]
**Concept:** A post-processing pipeline for applying screen-space effects like Grayscale and Scanlines to the framebuffer.
**Fate:** Merged
**Lesson:** Simple integer arithmetic (fixed-point luminance, bitwise darkening) is incredibly efficient for full-frame effects.

## [Depth Fog]
**Concept:** A z-buffer based fog effect that blends pixels with a fog color based on their depth.
**Fate:** Merged
**Lesson:** Simple post-processing can add significant depth to the scene using existing buffers.

## [3D Particles]
**Concept:** A lightweight particle system for adding dynamic visual effects like fire, smoke, and magic.
**Fate:** Merged
**Lesson:** Independent rendering pipelines can add complexity (duplicated projection logic) but allow for specialized behavior.

## [Mesh Modifiers]
**Concept:** A system for procedural geometry manipulation (twist, taper, noise) applied directly to mesh vertices.
**Fate:** In Progress
**Lesson:** TBD

## [Toon Outlines]
**Concept:** A post-processing effect that draws outlines by detecting discontinuities in the depth buffer, creating a "Toon" or "Technical Drawing" aesthetic.
**Fate:** Merged
**Lesson:** Depth buffers contain valuable structural information that can be reused for non-photorealistic rendering.
