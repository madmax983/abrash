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
**Fate:** Merged
**Lesson:** Modifying large meshes vertex-by-vertex can be computationally intensive, particularly for procedural displacement functions. Offloading iteration onto parallel CPU threads using Rayon greatly accelerates full-mesh deformations. Also, dynamically calculating or verifying per-vertex normals is critical, as out-of-bounds normal arrays can easily crash procedural adjustments.

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

## [Edge Glow Filter]
**Concept:** A neon-style screen-space post-processing effect that highlights edges using a configurable color and darkens non-edges. It effectively combines edge detection with dynamic tinting.
**Fate:** Implemented
**Lesson:** Using integer arithmetic and bitwise shifts combined with safe thread-local buffering (`thread_local!`) prevents dynamic allocation per frame while avoiding expensive floating-point overhead, leading to a massive performance speedup. Rayon `par_chunks_mut()` effectively handles independent rows.
## [Framebuffer Image Exporter]
**Concept:** Added zero-dependency TGA and PPM export functionality directly to the `Framebuffer` via an `ImageExporter` trait.
**Fate:** Implemented
**Lesson:** Writing uncompressed image formats natively (PPM for RGB, TGA for BGR) using simple bitwise extraction (`(pixel >> 16) & 0xFF`) and a `BufWriter` is extremely easy and removes the need for heavy external dependencies just to dump a visual artifact.

## [L-System Generator]
**Concept:** A procedural string-rewriting system (L-System) interpreted by a 3D Turtle to generate intricate branching structures (plants, fractals) directly into a `Mesh`.
**Fate:** Implemented
**Lesson:** Interpreting expanded strings using a stack-based state (Push/Pop orientation and position) is extremely powerful for generating recursive geometry like trees. To prevent OOM DoS attacks when expanding strings recursively, enforcing a strict char count capacity limit early inside the evaluation loop safely avoids excessive allocations and returns a graceful error.
## [Ascii Exporter]
**Concept:** A mashup feature extending the `Framebuffer` with an `AsciiExporter` trait that uses the existing `AsciiConverter`. It allows exporting any rendered frame to a `.txt` or colored `.ans` (ANSI) file directly, turning visual output into viewable text files via `cat`.
**Fate:** Implemented
**Lesson:** Simple additive trait implementations in experimental modules can safely combine existing systems (like the Framebuffer and AsciiConverter) into new, unexpected tooling.

## [Palette Swap / Retro Quantization]
**Concept:** A post-processing effect that maps the full 32-bit ARGB framebuffer to a predefined color palette (e.g. Gameboy, CGA, Vaporwave) using nearest-neighbor Euclidean distance in RGB space.
**Fate:** Implemented
**Lesson:** Iterating over the entire framebuffer and doing nearest neighbor distance checks against a small array (like 4-16 colors) is easily parallelizable with Rayon. Euclidean squared distance (`dr*dr + dg*dg + db*db`) avoids costly `sqrt` calculations in the inner loop.

## [Thermal Vision]
**Concept:** A post-processing effect that converts pixel luminance to a heatmap color palette, simulating a thermal imaging camera.
**Fate:** Implemented
**Lesson:** Extracting luminance and mapping it via threshold ranges into specific RGB blends creates a convincing thermal effect very efficiently without needing external LUT textures.

## [Boids Simulation]
**Concept:** A procedural flocking simulation based on Craig Reynolds' Boids algorithm implementing separation, alignment, and cohesion.
**Fate:** Implemented
**Lesson:** When implementing O(N^2) entity updates using parallel processing (like Rayon's `par_iter_mut`), avoid mutable aliasing errors by cloning the initial read-state (e.g., `let old_boids = self.boids.clone();`) and mapping over the mutable target array. Using `.hypot()` chained calls keeps distance calculations safe and clean.
