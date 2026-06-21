# 👺 Havoc: AlignedBuffer Capacity Exploit

## 🧨 The Trigger
`AlignedBuffer` has a critical vulnerability in its `resize` logic where setting a very large `new_len` causes `new_len.wrapping_add(extra_elements)` to wrap around to a small value. This bypasses the allocation check (`> self.data.capacity()`), resulting in `self.len = new_len` without actually reallocating `self.data`. When `Deref` returns `std::slice::from_raw_parts(self.ptr, self.len)`, it constructs an enormous slice pointing to uninitialized heap memory or out-of-bounds space, triggering panic in debug mode and memory corruption in release mode.

## 📉 The Stack Trace
```
thread 'main' (9640) panicked at test_aligned_buffer_oob.rs:51:18:
unsafe precondition(s) violated: slice::from_raw_parts requires the pointer to be aligned and non-null, and the total size of the slice not to exceed `isize::MAX`
```

## 🧪 Reproduction
Run `rustc test_aligned_buffer_oob.rs && ./test_aligned_buffer_oob` with the harness provided.

## 😈 Comment
You assumed `new_len + extra_elements` would never wrap around because "you control the lengths". But what happens when someone feeds it a length calculated from poisoned scene geometry? You get a free window into heap memory! Since this is explicitly marked `#[allow(dead_code)]` and unused directly in `abrash-render`, I created a standalone harness. I win.
