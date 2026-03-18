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

## [Water Ripple Filter]
**Concept:** A post-processing effect that creates dynamic liquid surfaces by applying a radial sine-wave displacement to the framebuffer to simulate a water droplet ripple.
**Fate:** Implemented
**Lesson:** Cloning the source framebuffer is necessary for non-linear pixel displacement to avoid aliasing issues when parallelizing with Rayon. Replacing `round()` with fast float-to-int casts (`as i32`) prevents inner-loop bottlenecks when evaluating the sine waves per pixel.

## [Anaglyph 3D]
**Concept:** A post-processing effect that generates stereoscopic 3D images by shifting the red channel horizontally based on Z-buffer depth.
**Fate:** Implemented
**Lesson:** Shifting color channels based on depth information requires a forward-write or careful reverse-lookup algorithm because multiple source pixels might attempt to shift their red value to the same destination pixel depending on depth layering. It is crucial to determine a "winning" pixel (e.g., the one closest to the camera) for each coordinate to avoid visual artifacts. Thread-local row buffers are required when processing with Rayon to safely access row data without reallocation overhead.

## [Autostereogram Generator]
**Concept:** A single-image stereogram (Magic Eye) generator from a depth map (ZBuffer).
**Fate:** Merged
**Lesson:** Using a union-find-like approach to link pixels horizontally by depth shift efficiently prevents recursive lookbacks and allows for perfect row-by-row parallelization.

## [Chromatic Aberration Filter]
**Concept:** A retro post-processing effect that simulates camera lens imperfections by shifting the red, green, and blue color channels independently, creating a colored fringe effect.
**Fate:** Implemented
**Lesson:** Safely applying directional shifts to color channels without mutating the data being read requires cloning the original framebuffer to use as a source texture, while Rayon easily processes the destination buffer.

## [Vignette Filter]
**Concept:** A retro post-processing effect that darkens the edges of the screen, creating a cinematic or CRT-like focus towards the center.
**Fate:** Implemented
**Lesson:** Treating distance in UV coordinate space (0.0 to 1.0) instead of raw pixels simplifies aspect-ratio scaling for elliptical vignettes. Precalculating inverse dimensions and using fast smoothstep equations (`3t^2 - 2t^3`) significantly optimizes the inner loop when parallelized with Rayon.

## [Voronoi Filter]
**Concept:** A post-processing effect that calculates distance from random seeds to create a stained-glass or cellular look. Features configurable metric distance (Euclidean vs Manhattan) and border outlines based on distance comparisons.
**Fate:** Implemented
**Lesson:** Generalizing Minkowski distance with arbitrary exponents inside a tight per-pixel loop using `powf` is very slow. Implementing fast paths for `metric == 1.0` and `metric == 2.0` avoids exponentiation and greatly speeds up the effect. Computing border thickness accurately requires finding the difference between the closest and second-closest seed distances. Parallelization via Rayon makes processing the image row-by-row efficient.

## [Swirl Filter]
**Concept:** A retro post-processing effect that distorts the image by displacing pixels radially based on their distance from a center point, creating a localized swirl or pinch effect.
**Fate:** Implemented
**Lesson:** Cloning the source framebuffer (`fb.as_slice().to_vec()`) is critical to safely parallelize non-linear pixel sampling with Rayon without mutable aliasing. Additionally, calculating `distance2 < radius2` avoids an expensive `sqrt` call for the vast majority of non-affected pixels, and using fast float-to-int casts (`as i32`) prevents inner-loop bottlenecks. Precalculating `1.0 / radius` allows for multiplication instead of division in the inner loop.

## [Wobble Filter]
**Concept:** A retro post-processing effect that distorts the image by displacing pixels horizontally based on a sine wave of their Y-coordinate, simulating classic SNES-style underwater or heat haze effects.
**Fate:** Implemented
**Lesson:** Cloning the source framebuffer (`fb.as_slice().to_vec()`) is required to safely parallelize non-linear pixel lookups using Rayon without mutable aliasing. Using fast float-to-int casts (`as i32`) is beneficial for the inner loop.
## [Blueprint Filter]
**Concept:** A post-processing effect that transforms the framebuffer into an architectural/engineering blueprint by applying edge detection and mapping light pixels to white/cyan lines while replacing the background with a deep blue color and overlaying a faint engineering grid.
**Fate:** Implemented
**Lesson:** Combining existing effects like `edge_glow` with a custom post-processing color map and grid drawing efficiently achieves a stylistic look without needing complex edge detection algorithms from scratch. Iterating over the image in chunks with Rayon provides great parallel performance.

## [Tilt-Shift Filter]
**Concept:** A retro post-processing effect that blurs the top and bottom of the image while keeping a central focal band sharp, simulating a miniature faking effect.
**Fate:** Implemented
**Lesson:** Using a thread-local context with zero-initialized buffers (`Vec::resize`) allows zero-allocation per frame execution. Leveraging separable box blur and a smoothstep transition (`t * t * (3.0 - 2.0 * t)`) effectively models depth of field transitions. Rayon's `par_chunks_exact_mut` allows fast per-row blending against the pre-blurred image slice.

## [Doom Fire Effect]
**Concept:** Implemented the classic Doom fire cellular automata algorithm. Heat propagates upwards with random decay and horizontal drift.
**Fate:** Implemented
**Lesson:** Simple 1D heat buffers with a 36-color fiery palette overlaid on the bottom of a framebuffer can produce very convincing classic retro fire without complex physics simulations.
