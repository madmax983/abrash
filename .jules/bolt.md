**Eliminate dynamic heap zero-initialization in 1D convolution**
**Learning:** Initializing large output vectors with `vec![0.0; len]` performs a dynamic heap allocation followed by a `memset` zero-initialization. In math functions like `convolve_1d` where every element is immediately overwritten anyway, this zeroing-out pass is purely wasted overhead.
**Action:** Replace `vec![0.0; len]` with `Vec::with_capacity(len)` and compute the final element values directly using `.push()`. This achieves the exact same mathematical result with true zero-cost memory initialization.

