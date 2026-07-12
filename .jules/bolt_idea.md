I noticed in memory `Bolt Learning: In extremely hot loops processing arrays within mathematically guaranteed bounds (e.g., scanline rasterization tails or image downsampling), safe slice indexing (like buffer[i..len].iter_mut()) incurs redundant bounds check overhead. Replacing these with unsafe { buffer.get_unchecked_mut(...) } safely bypasses this overhead, yielding measurable performance improvements.`

Let's check `crates/abrash-core/src/rasterizer/scanline.rs` or `crates/abrash-core/src/rasterizer/flat.rs`
