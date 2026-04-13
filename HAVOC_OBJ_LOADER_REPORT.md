# 👺 Havoc: `load_obj` Input Validation Fuzzer

### 🧨 The Trigger
An unbounded, malformed Wavefront `.obj` file (or specifically crafted long vertex indices) throws internal array out-of-bounds reads and arithmetic overflows during integer string parsing.

### 📉 The Stack Traces

1. **Integer Overflow Panic:**
```
thread '<unnamed>' panicked at crates/abrash-core/src/obj_loader.rs:396:20:
attempt to add with overflow
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

2. **Array Out of Bounds Panic:**
```
thread '<unnamed>' panicked at crates/abrash-core/src/obj_loader.rs:295:34:
index out of bounds: the len is 1000000 but the index is 1048575
```

### 🧪 Reproduction
Run `cargo fuzz run fuzz_target_1` in the `crates/abrash-core` directory.

### 😈 Comment
You assumed the buffer size bounds implicitly via bitwise constraints without explicit capacity length limit checks. You also assumed `usize` overflow would never happen on a perfectly-crafted 19 digit string. You were wrong.
