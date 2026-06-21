cargo bench --bench blitter_bench --features gpu-render || true
cargo bench --bench draw_list_bench || true
cargo bench --bench culling || true
cargo bench --bench gpu_render --features gpu-render || true
