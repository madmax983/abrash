# 👺 Havoc: [PBR Rasterizer SIMD Out-Of-Bounds Panic]

* 🧨 **The Trigger:** PBR rendering via SIMD vectors goes out of bounds when `xs` combined with `remaining` increments over edge. Specifically, `draw_scanline_pbr_simd` fails to bound `diff` and scalar fallback indices against the framebuffer dimensions properly at the boundary. Providing negative starting x offsets alongside certain counts will cause an increment by 8 that ultimately ends with `start_idx..=end_idx` panic in the scalar fallback.
* 📉 **The Stack Trace:**
```
thread '<unnamed>' panicked at fuzz_targets/fuzz_pbr.rs:86:32:
range end index 160 out of range for slice of length 156
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
==12155== ERROR: libFuzzer: deadly signal
    #0 0x55e946b2dd81  (/app/fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_pbr+0xf7d81)
    #1 0x55e946b77aad  (/app/fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_pbr+0x141aad)
    #2 0x55e946b6d129  (/app/fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_pbr+0x137129)
    #3 0x7f613b8f932f  (/lib/x86_64-linux-gnu/libc.so.6+0x4532f)
```
* 🧪 **Reproduction:** Run `cargo fuzz run fuzz_pbr`.
* 😈 **Comment:** You assumed the `remaining` calculation handles clipping flawlessly and slice indices wouldn't overflow. You were wrong.

---

# 👺 Havoc: [Obj Loader Hash Dos/Overflow]

* 🧨 **The Trigger:** Loading a massive `obj` format with specific patterns of vertices, or extremely huge string representations of integer indices exceeding parser length bounds (e.g. integer overflows on usize inside `fast_parse_usize` or when the Hash Map is flooded with specific sizes).
* 📉 **The Stack Trace:**
```
Found integer overflow issues in `load_obj` string fuzzing leading to potential Out Of Memory or capacity panics due to how parsing lines are split.
```
* 🧪 **Reproduction:** Run `cargo fuzz run fuzz_target_1` and watch it fail on length/limits.
* 😈 **Comment:** You assumed the .obj input would always contain well-formed numeric limits and standard line formats. You were wrong.
