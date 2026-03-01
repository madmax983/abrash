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

## [Procedural Terrain]
**Concept:** A procedural terrain generator using Value Noise to create Meshes.
**Fate:** Merged
**Lesson:** Isolating experimental features in `src/experimental` allows for rapid prototyping without affecting core logic.

## [Screen-Space Ambient Occlusion (SSAO)]
**Concept:** Simulates global illumination by darkening crevices based on the depth buffer.
**Fate:** Merged
**Lesson:** Reconstructing view-space position from the depth buffer allows for advanced lighting effects without heavy ray tracing, though software implementation requires careful optimization.

## [Isosurface Extraction]
**Concept:** A system to convert mathematical Signed Distance Fields (SDFs) into renderable triangle meshes using the Marching Tetrahedra algorithm.
**Fate:** Merged
**Lesson:** Standard Marching Cubes tables found online often use different vertex/edge ordering conventions; deriving the 16 cases of Marching Tetrahedra manually is more reliable and ensures topological correctness.

## [Pixel Sort]
**Concept:** A glitch art effect that sorts pixels along rows or columns based on luminance thresholds, creating a melting or tearing aesthetic.
**Fate:** Implemented
**Lesson:** Applying 1D sorting algorithms directly on segmented arrays using Rust's slice grouping capabilities creates compelling visual chaos with surprisingly few lines of code.
## [Kuwahara Filter]\n**Concept:** A non-photorealistic post-processing filter that calculates the mean and variance of colors in four overlapping regions around each pixel, assigning the mean color of the region with the lowest variance. This creates a painterly, oil-painting-like aesthetic while preserving hard edges.\n**Fate:** Implemented\n**Lesson:** The Kuwahara filter provides a great example of an algorithm that operates on overlapping regions to smooth out noise without destroying edges. It effectively brings an artistic "painting" effect to retro rendering.
