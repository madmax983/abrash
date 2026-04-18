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

## [Joy Division Topology Filter]
**Concept:** A post-processing effect that converts the screen into a series of horizontal waveform lines. It samples the luminance of the underlying framebuffer and displaces the horizontal lines upwards based on the brightness, creating a 3D-like topographical mountain effect using pure 2D rasterization, similar to the iconic album cover.
**Fate:** Implemented
**Lesson:** Iterating from top to bottom (back-to-front in terms of displacement) and filling the column below each displaced peak with the background color acts as an efficient painter's algorithm, perfectly masking the lines drawn in previous loop iterations.
