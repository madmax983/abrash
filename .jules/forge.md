**Refactoring God Functions in TUI Apps**
**Learning:** Monolithic UI drawing functions in `ratatui` (like a 150-line `run_app` layout block) are common "God functions." They can be seamlessly extracted into smaller, focused helper functions (e.g., `render_title`, `render_demo_list`) by passing `&mut ratatui::Frame` and the specific layout chunk (`ratatui::layout::Rect`). This dramatically flattens the pyramid of doom without triggering borrow checker conflicts or changing runtime behavior.
**Action:** When working on UI code with deep nesting and multiple layout blocks, proactively extract each visual component into its own function taking `f: &mut Frame` and `area: Rect`.

**[Fix SIMD Segfaults]**
**Learning:** `_mm256_store_ps` and `_mm256_store_si256` require 32-byte memory alignment. Using them on buffers that aren't perfectly aligned causes a segmentation fault (SIGSEGV). In tile-based renderers or dynamically sliced buffers, pointers may lose 32-byte alignment.
**Action:** Replace `_mm256_store_ps` with `_mm256_storeu_ps` and `_mm256_store_si256` with `_mm256_storeu_si256` to perform unaligned stores safely and prevent crashes.
