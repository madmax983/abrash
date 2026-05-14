## ⚡ Bolt: thread_local buffer optimization for texture uploads

**Learning:** In wgpu texture upload operations (like `upload_framebuffer` or `create_texture`), allocating a zeroed vector `vec![0u8; len * 4]` every frame causes severe O(W*H) heap allocation and initialization overhead, hindering performance in tight render loops.
**Action:** Replace `vec![0u8; ...]` with a dynamically resized `thread_local!` `RefCell<Vec<u8>>` buffer. This eliminates per-frame heap allocations while preserving the LLVM vectorization benefits of the subsequent `.chunks_exact_mut()` loop, significantly improving throughput for batch processing and frame uploads.
