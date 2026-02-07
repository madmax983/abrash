**[Dead Code in Renderer]**
**Learning:** Legacy SIMD and fixed-point implementations were left in the codebase but unused by the active rendering path, creating confusion and maintenance burden.
**Action:** When disabling optimization paths (like SIMD) due to performance regressions, remove the code entirely or move it to a dedicated "experimental" branch/module, rather than commenting it out or leaving it dead.

**[Refactoring Complex Parsing Logic]**
**Learning:** The `load_obj` function was a "God Function" mixing parsing, state management, and error handling.
**Action:** Extracting state into a helper struct (`ObjParser`) with dedicated methods (`parse_vertex`, etc.) significantly improved readability and testability without changing the public API.
