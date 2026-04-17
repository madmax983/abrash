[Eliminate O(N) heap allocations during ASCII generation and file export]
**Learning:** `fmt::Display` and file export methods like `export_ascii` previously constructed entire text outputs in memory by allocating `String`s sized relative to the framebuffer (e.g., millions of characters) and appending them pixel-by-pixel.
**Action:** Replaced massive string allocations with direct `f.write_char(...)` inside `fmt::Display`, and implemented streaming file writes using `BufWriter` for `export_ascii` and `export_ansi` to write data chunk-by-chunk without intermediate allocations.
