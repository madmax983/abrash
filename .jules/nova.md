## Procedural String Art
**Concept:** A procedural generator that simulates string woven between pegs on a circle, forming an image matching a darkness map.
**Fate:** Implemented using a score-based evaluation system iterating over a pixel array. Optimized to pre-calculate line cache indices to avoid recursive recalculation.
**Lesson:** When building algorithms requiring repeated evaluations of specific pixel paths (e.g. evaluating 1D pixels from Bresenham's line algorithm), flattening variable-length pixel line arrays into a single `Vec<usize>` and maintaining an offsets cache greatly reduces evaluation time, yielding a >64% improvement by avoiding O(N^2) memory allocations.

## Procedural String Art
**Concept:** A procedural generator that simulates string woven between pegs on a circle, forming an image matching a darkness map.
**Fate:** Implemented using a score-based evaluation system iterating over a pixel array. Optimized to pre-calculate line cache indices to avoid recursive recalculation.
**Lesson:** When building algorithms requiring repeated evaluations of specific pixel paths (e.g. evaluating 1D pixels from Bresenham's line algorithm), flattening variable-length pixel line arrays into a single `Vec<usize>` and maintaining an offsets cache greatly reduces evaluation time, yielding a >64% improvement by avoiding O(N^2) memory allocations.
