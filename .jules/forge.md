**[Clippy fixes in abrash-render and abrash-gpu-render]
**Learning:** `clippy::match-same-arms`, `clippy::missing-const-for-fn`, `clippy::doc-markdown`, `clippy::must-use-candidate`, `clippy::too-many-lines`, `clippy::unnecessary-map-or`, `clippy::pub-underscore-fields`, `clippy::unused-self`
**Action:** Always run clippy and fix warnings.

**[Clippy fixes in abrash-render and abrash-gpu-render]
**Learning:** Extracting complex functions like pipeline constructors into smaller methods (e.g. `create_frame_layout`, `create_gbuffer_layout`) improves readability and adheres to the persona's core goals without suppressing `too_many_lines` warnings. Also learned that `FLOAT32_BLENDABLE` is a necessary wgpu feature for our deferred pipeline to function on `Rgba32Float` formats.
**Action:** Extract large methods into helper functions instead of suppressing warnings. Ensure we only use `#[allow(dead_code)]` when variables or struct definitions are truly intentionally unused.
