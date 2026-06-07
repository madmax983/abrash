**[Havoc AsciiConverter Overflow]**
- **Trigger:** `AsciiConverter::to_colored_string()` allocates capacity using `(((width * 20) * height) as usize)`, which overflows when `width` and `height` are very large (but still fit in u32, e.g., width 214,748,365 and height 1). The 32-bit math wraps around *before* being cast to usize, leading to an extremely undersized allocation and subsequent massive reallocations/OOM.
- **Payload:** Fuzzing with `proptest` within bounds.
