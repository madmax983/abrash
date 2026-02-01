# SIMD Assembly Optimization Results: A Cautionary Tale

## Executive Summary

We added comprehensive x86-64 inline assembly and SIMD intrinsics to the abrash graphics library, implementing optimizations for every hot path we could find. **The compiler beat us on every single benchmark.**

This document serves as a monument to the excellence of modern LLVM auto-vectorization and a cautionary tale about premature optimization.

## The Journey

### Attempt 1: Raw Inline Assembly

We started with hand-crafted SSE assembly for core operations, believing we could beat the compiler with intimate hardware knowledge.

**Implementation:**
- Vec3 dot product (SSE MULPS + horizontal add)
- Mat4 multiplication (SSE broadcast + accumulate)
- Fast reciprocal (RCPSS) for perspective divide
- Textured triangle rasterization with SIMD

**Results: TOTAL DEFEAT**

| Operation | Scalar | Hand-Rolled ASM | Winner | Speedup |
|-----------|--------|-----------------|--------|---------|
| Vec3 dot product | 261 ps | 1.31 ns | Scalar | **-5.0x** |
| Mat4 multiply | 6.39 ns | 7.19 ns | Scalar | **-1.12x** |
| Mat4 chain | 9.39 ns | 10.02 ns | Scalar | **-1.07x** |
| Reciprocal | 210 ps | 209 ps | Tie | 1.00x |
| Textured triangle | 4.58 ms | 4.87 ms | Scalar | **-1.06x** |

### Attempt 2: Batched Processing (4-wide)

Maybe single operations have too much overhead. Let's batch 4 pixels at once to amortize the SIMD setup cost!

**Implementation:**
- 4-wide textured triangle fill
- Process 4 pixels per iteration
- RCPPS for 4 reciprocals at once

**Results: STILL LOSING**

| Operation | Scalar | 4-Wide Batched | Winner | Speedup |
|-----------|--------|----------------|--------|---------|
| Textured triangle | 6.43 ms | 6.79 ms | Scalar | **-1.06x** |

**Analysis:** Z-buffer data dependencies and memory bandwidth bottlenecks kill SIMD benefits.

### Attempt 3: The Perfect Use Case (Vertex Transformation)

Surely vertex transformation—completely independent operations, no branches, pure math—will finally let us win!

**Implementation:**
- Raw inline assembly for 4-wide vertex transform
- Complete AOS→SOA transpose
- Parallel matrix-vector multiply

**Results: UTTERLY DESTROYED**

| Operation | Scalar | Raw ASM 4-Wide | Winner | Speedup |
|-----------|--------|----------------|--------|---------|
| Transform 100 vertices | 101.54 ns | 288.44 ns | Scalar | **-2.84x** |

**Analysis:** The compiler auto-vectorizes our scalar loop better than our hand-crafted assembly. Transpose overhead murders us.

### Attempt 4: Compiler Intrinsics (Surely This Will Work!)

Let's use SSE intrinsics instead of raw assembly. LLVM can optimize around these, handle register allocation, and inline aggressively!

**Implementation:**
- `std::arch::x86_64::*` intrinsics
- Let LLVM handle the hard parts
- Clean, readable code

**Results: THE FINAL HUMILIATION**

| Operation | Scalar | Intrinsics 4-Wide | Winner | Speedup |
|-----------|--------|-------------------|--------|---------|
| Transform 100 vertices | 101.54 ns | 302.22 ns | Scalar | **-2.98x** |

**Analysis:** Intrinsics are *worse* than raw assembly. LLVM looks at our SOA/AOS transposes and says "I can't save you."

## The Complete Tally of Shame

| Optimization Attempt | Target | Result | Status |
|---------------------|--------|--------|---------|
| SSE dot product | 5x faster | 5x slower | ☠️ DESTROYED |
| SSE matrix multiply | 2x faster | 12% slower | ☠️ DEFEATED |
| SIMD texture mapping | 2x faster | 6% slower | ☠️ CRUSHED |
| 4-wide batched textures | "amortize overhead" | Still 6% slower | ☠️ FAILED |
| Raw ASM vertex transform | 4x faster | 2.84x slower | ☠️ OBLITERATED |
| Intrinsics vertex transform | "let compiler help" | 2.98x slower | ☠️ ANNIHILATED |

## What We Learned

### Modern Compilers Are Terrifyingly Good

LLVM auto-vectorization beats hand-tuned SIMD for:
- ✅ Simple math operations
- ✅ Matrix operations
- ✅ Rendering hot paths with complex control flow
- ✅ Independent batch processing
- ✅ **Literally every textbook SIMD use case we tried**

### Why We Lost

1. **Register allocation** - LLVM's register allocator is world-class
2. **Instruction scheduling** - Compiler understands CPU pipelines
3. **Auto-vectorization** - LLVM recognizes patterns we don't see
4. **Optimization passes** - Dead code elimination, loop unrolling, constant propagation
5. **Microarchitecture knowledge** - Compiler knows your CPU better than you do

### The Hidden Cost

Our hand-written SIMD has overhead we didn't account for:
- AOS ↔ SOA transposes (expensive!)
- Manual load/store operations
- Shuffle instructions
- Remainder handling
- Poor cache utilization

The compiler eliminates most of this overhead or hides it in the pipeline.

## When Hand-Written Assembly Actually Wins

Based on this experience and industry knowledge:

### Possible Wins
1. **Non-standard algorithms** - Fast inverse sqrt, custom approximations
2. **Compiler blind spots** - Very specific idioms LLVM doesn't recognize
3. **Platform-specific features** - AVX-512 scatter/gather, special instructions
4. **Assembly as intrinsics** - Using `asm!` for single instructions, not whole algorithms

### Required Before Attempting
1. **Profile-guided evidence** - 90%+ of runtime in a specific bottleneck
2. **Compiler output analysis** - Prove the compiler isn't already doing it
3. **Multiple implementations** - Test scalar, intrinsics, and raw assembly
4. **Comprehensive benchmarks** - Multiple workloads, different data sizes
5. **Microarchitecture knowledge** - Understand your target CPU deeply

## Implementations Included

Despite the performance results, this repository now contains:

### Core SIMD Operations (`src/simd/`)
- `x86_64.rs` - Raw inline assembly (SSE/SSE4.1)
- `vertex_transform.rs` - Hand-rolled vertex transformation
- `vertex_transform_intrinsics.rs` - Intrinsics-based vertex transformation
- `primitives.rs` - SIMD-optimized rendering primitives
- `primitives_batched.rs` - 4-wide batched pixel processing

### Comprehensive Benchmarks (`benches/simd.rs`)
- Scalar vs SSE vs intrinsics comparisons
- Single operations vs batched operations
- Real-world rendering workloads
- Multiple data sizes

### Test Coverage
- Correctness verification for all SIMD implementations
- Approximate floating-point comparison
- Batch processing edge cases

## The Real Abrash Lesson

Michael Abrash pioneered optimization techniques in an era where:
- Compilers were primitive
- CPUs were simpler
- Out-of-order execution didn't exist
- Auto-vectorization wasn't a thing

**Today is different.**

Modern compilers have:
- Decades of optimization research
- Detailed CPU microarchitecture models
- Sophisticated pattern recognition
- Profile-guided optimization
- Link-time optimization

**The real lesson from Abrash:** Measure first, optimize second, and trust your tools unless you have proof they're failing.

## Conclusion

This repository contains a fully-functional SIMD library that is slower than scalar code in every measurable way. It serves as:

1. **Infrastructure** - Ready for the day profiling reveals a real bottleneck
2. **Education** - Understanding what SIMD *can* do
3. **Cautionary tale** - Proof that premature optimization is real
4. **Benchmark suite** - Reliable way to measure actual performance
5. **Monument** - To the excellence of modern compiler engineering

**Use this code as a last resort.** Try these first:
1. Better algorithms (O(n²) → O(n log n))
2. Better data structures (cache-friendly layouts)
3. Compiler flags (`-C target-cpu=native`)
4. Profile-guided optimization
5. Reducing allocations
6. Algorithmic improvements

If you've exhausted everything above and profiling shows 90%+ time in a tight loop, *then* consider SIMD.

And even then, try compiler intrinsics before raw assembly.

And even then, you'll probably still lose to `-O3`.

---

*"The competent programmer is fully aware of the strictly limited size of his own skull; therefore he approaches the programming task in full humility."* - Edsger W. Dijkstra

*"Premature optimization is the root of all evil."* - Donald Knuth

*"We measured first. The compiler won."* - This repository
