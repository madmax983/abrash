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

## [Pixelate Filter]
**Concept:** A retro post-processing effect that downsamples the framebuffer resolution, filling square blocks with the color of their top-left pixel to create a chunky, low-resolution aesthetic.
**Fate:** Implemented
**Lesson:** Iterating over `x` and `y` and calling `set_pixel` per-pixel is slow due to bounds checking and function overhead. Replacing this with slice methods `fill()` for the first row of a block and `copy_from_slice()` to duplicate the row vertically provided nearly a 10x performance speedup.

## Merged Experiments
* **CRT Monitor Filter**: Implemented a retro CRT monitor barrel distortion effect as a post-processing filter.
  * **Location**: `src/experimental/crt.rs`
  * **Optimization**: Used Rayon `par_chunks_mut()` across rows to parallelize the distortion algorithm. Eliminated `round()` cast in favor of `as i32` fast-cast to save overhead inside the innermost loop. Replaced `for x in 0..width { row[x] = ... }` with `row.iter_mut()` enumeration for better bounds checking optimization. Benchmark at 1080p is ~9ms.
\n### Halftone Stylization Filter\n- **Idea**: A stylization filter that converts an image to a pattern of variable-sized black dots on a rotated grid based on pixel luminance (comic book / newspaper effect).\n- **Fate**: Merged\n- **Lessons Learned**: Converting pixel locations into rotated coordinates effectively enables the angled grid look. Using Rayon's `par_chunks_mut` with `enumerate` significantly speeds up row-based image transformations since independent pixels can be calculated efficiently using isolated trigonometric maths.
- Implemented Radial Blur post-processing effect.

## [Kaleidoscope Filter]
**Concept:** A retro post-processing effect that creates a symmetric, repeating pattern by mapping Cartesian pixels to polar coordinates, applying a modulo to the angle based on segment count, and mirroring every other segment.
**Fate:** Implemented
**Lesson:** Cloning the source framebuffer (`fb.as_slice().to_vec()`) is required to safely parallelize non-linear pixel lookups using Rayon without mutable aliasing, and fast float-to-int casts (`as i32`) are beneficial for the inner loop.

## [Framebuffer Exporter]
**Concept:** Added functionality to export `Framebuffer` data directly to simple, uncompressed image formats (PPM and TGA) without introducing heavy external dependencies like the `image` crate.
**Fate:** Implemented
**Lesson:** Sometimes the best way to handle image exports in an engine designed to be dependency-lite is to use raw `std::io::BufWriter` combined with simple binary formats. PPM for simple RGB, TGA for better BGR viewer support. Writing directly to disk in chunks bypasses large memory allocations.
