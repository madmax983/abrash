# 👺 Havoc: Massive Allocation Out-Of-Memory DOS

## 🧨 The Trigger
Calling struct constructors with maximum dimensions directly allocates `width * height` array elements. While dimension boundaries are guarded by `i32::MAX`, the pixel count limit only prevents strict arithmetic overflows, permitting `u32::MAX` total elements. This leads to requesting contiguous memory blocks of over 17 gigabytes.

```rust
// Allocates 17,179,344,900 bytes and immediately panics!
Framebuffer::new(i32::MAX as u32, 2u32);
ZBuffer::new(u32::MAX / 2, 2u32);
Texture::new(65535, 65535);
```

## 📉 The Stack Trace
```
memory allocation of 17179344900 bytes failed

Caused by:
  process didn't exit successfully: `/app/target/debug/deps/havoc_proptest-061306f38d060687` (signal: 6, SIGABRT: process abort signal)
```

## 🧪 Reproduction
Run the intentionally ignored proptest suite.
```bash
cargo test --test havoc_proptest -- --ignored
```

## 😈 Comment
You assumed developers would never request a buffer larger than their available system RAM. Or perhaps you thought validating dimensions meant validating the heap. You were wrong. I just crashed the whole test runner process.
