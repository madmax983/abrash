## [ASCII Display Filter]
**Concept:** A post-processing effect that converts the framebuffer into an ASCII art display, mapping luminance to characters and rendering them using a built-in bitmap font.
**Fate:** Implemented
**Lesson:** Rendering simple pixel-art text efficiently across a framebuffer requires avoiding naive scaling operations or complex string allocations, and instead directly mapping pixel locations and checking simple bitmaps using shifts and masks.

## [Pencil Sketch Filter]
**Concept:** A post-processing effect that simulates a hand-drawn pencil sketch. It works by combining edge detection (to draw the strokes) with noise-driven hatching to simulate shading and texture.
**Fate:** Proposed
**Lesson:** TBD

## [Pencil Sketch Filter]
**Concept:** A post-processing effect that simulates a hand-drawn pencil sketch. It works by combining edge detection (to draw the strokes) with noise-driven hatching to simulate shading and texture.
**Fate:** Implemented
**Lesson:** Using procedural coordinate-based hashing combined with luminance is a highly effective and very fast way to produce stylized noise patterns like hatching, completely avoiding the overhead of external random number generator libraries in tight hot loops.
**Lesson:** Iterating from bottom-to-top avoids teleporting particles through multiple steps in a single frame. Randomizing horizontal processing direction prevents directional bias when sand grains fall diagonally.

## [LED Matrix Filter]
**Concept:** A post-processing effect that converts the image into an LED matrix display, grouping pixels into cells and drawing a glowing circular LED for each cell to simulate a jumbotron or pixel display.
**Fate:** Implemented
**Lesson:** Grouping pixels into cells and rendering geometric shapes (circles) based on the sampled center pixel effectively simulates hardware displays. Cloning the source framebuffer prevents read/write aliasing during parallel processing with Rayon.

## [Speed Lines Filter]
**Concept:** A procedural post-processing effect that simulates anime/comic style radial speed lines using a fast polar-coordinate procedural hash function.
**Fate:** Implemented
**Lesson:** Using simple (x, y) wrapping hashes combined with atan2 provides a fast and highly stylizable procedural generation method without the heavy overhead of full noise libraries.

## [Depth Fog Filter]
**Concept:** A post-processing effect that applies atmospheric distance fog to scenes by blending the rendered pixels with a fog color based on the depths stored in the Z-buffer.
**Fate:** Implemented
**Lesson:** By reading directly from the Z-buffer during the post-processing stage, realistic depth-based effects can be achieved independently of the vertex shading pass, keeping the rasterizer decoupled from atmospheric calculations while still yielding an immersive sense of scale.

## [Topography Filter]
**Concept:** A post-processing effect that converts an image into horizontal waveforms based on pixel luminance, simulating a 3D topographical map or the iconic 'Joy Division - Unknown Pleasures' aesthetic.
**Fate:** Implemented
**Lesson:** Iterating from top-to-bottom (back-to-front) and masking out the area below the displaced scanlines using a painter's algorithm effectively creates layered depth and occlusion without the need for a z-buffer. Squaring the luminance mapping helps create sharper peaks.

## [Sonar / Echolocation Filter]
**Concept:** A post-processing effect that visualizes depth as sweeping sonar waves radiating from the camera over time, mapped directly from the Z-Buffer values.
**Fate:** Implemented
**Lesson:** Using simple math like scaling and trigonometric functions (sine) on depth values creates an extremely convincing procedural mapping for sonar waves. Darkening geometry based on its distance effectively merges the geometry into a solid background, creating an eerie atmosphere completely in screen-space.

## [Harmonograph Generator]
**Concept:** A mathematical renderer that simulates a mechanical harmonograph using damped pendulums to draw complex Lissajous curves and geometric patterns directly onto the 2D framebuffer.
**Fate:** Implemented
**Lesson:** Using a combination of simple trigonometric functions (sine) mapped to exponentially decaying amplitudes (damping) creates beautiful procedural curves. Bresenham's line algorithm effectively rasterizes these continuous math functions without requiring float-to-int pixel interpolation artifacts if sampled frequently enough (small step sizes).

## [Cel Shading (Toon Shading) Filter]
**Concept:** A post-processing effect that simulates a comic book or anime aesthetic by combining color quantization (flat shading) with Sobel edge detection over depth and luminance.
**Fate:** Implemented
**Lesson:** Combining edge detection on the Z-Buffer (for object outlines) and the Luminance map (for internal creases/details) creates a highly robust cel-shaded look purely in screen-space, avoiding the need for custom forward shaders while effectively mimicking toon-style rendering.

## [Conway's Game of Life Filter]
**Concept:** A post-processing cellular automata filter based on Conway's Game of Life. It uses the framebuffer's luminance to seed the game state, causing bright areas of the scene to dissolve into an evolving, interactive cellular pattern.
**Fate:** Implemented
**Lesson:** Maintaining persistent state across frames for a post-processing filter can be achieved effectively by using `thread_local!` `RefCell` buffers. This allows complex automata like Conway's Game of Life to interact dynamically with the 3D scene being rendered without passing state manually through the main pipeline.

## [Cross-Stitch Filter]
**Concept:** A retro-style post-processing effect that converts the framebuffer into a pattern resembling a cross-stitch embroidery canvas, dividing the image into cells and drawing an 'X' with absolute coordinates on a canvas background.
**Fate:** Implemented
**Lesson:** Using basic modulo arithmetic and absolute coordinate differences within standard iteration loops provides an efficient, dependency-free alternative to calling external geometric rendering functions when making simple pixel-art grid patterns.

## [Hexagonal Mosaic Filter]
**Concept:** A post-processing effect that converts the framebuffer into a hexagonal grid mosaic ("Hexagons are the bestagons"), finding the nearest hex center and optionally drawing a stylish border.
**Fate:** Implemented
**Lesson:** Iterating over grids with non-orthogonal axes and calculating the nearest point between two interlocking rectangular grids is mathematically much faster than naive geometry algorithms. Checking distance thresholds from the center allows for quick border extraction.
