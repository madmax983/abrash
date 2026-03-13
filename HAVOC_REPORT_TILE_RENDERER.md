# 👺 Havoc: TileRenderer Mesh Submission Panic

## 🧨 The Trigger
Calling `TileRenderer::submit_mesh` with `indices` that reference out-of-bounds `vertices` causes a hard panic. The `indices` are not validated against the `vertices` slice length. An array of length 3 passed an index of `3` or higher, breaking the program.

## 📉 The Stack Trace
```text
thread 'test_tile_renderer_panic' panicked at src/rasterizer/tile.rs:1807:26:
index out of bounds: the len is 3 but the index is 3
```

## 🧪 Reproduction
Run `cargo test --test havoc_tile_renderer_panic` using the proptest file that submits random out-of-bounds indices `> 3`.

## 😈 Comment
You trusted the user to always provide valid geometry indices. Never trust the user. If the model loader returns bad data, your entire rasterizer thread blows up instead of gracefully dropping the triangle or returning an error. I win.