# Forge's Journal

## Critical Learnings

**[Tuple Obsession in Pipeline]**
**Learning:** The pipeline was passing raw tuples `(i32, i32, f32)` and manual closures for sorting, leading to duplicated logic and hard-to-read signatures.
**Action:** Extract domain structs like `ScreenPoint` and sorting helpers early to prevent "Primitive Obsession" from spreading.
