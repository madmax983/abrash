# Software Renderer with Win32 - Design Document

**Date:** 2026-01-13
**Status:** Approved
**Goal:** Build a software renderer in Rust inspired by Michael Abrash's Graphics Programming Black Book, starting with 2D primitives and progressing to 3D, with modern SIMD optimizations.

## Overview

This project implements a layered graphics system from first principles:
- Software rendering (no wgpu) to understand rasterization fundamentals
- Win32 platform layer for windowing (extensible to other platforms later)
- Custom framebuffer management
- Hand-rolled math library with SIMD optimizations
- TDD approach with comprehensive benchmarks
- Foundation for future Vulkan learning

## Architecture

### Layer 1: Platform (Win32)

**Responsibility:** Window creation, event handling, and framebuffer blitting.

**Implementation:**
- `CreateWindowExW` with custom window class via `RegisterClassW`
- Message loop using `GetMessageW`/`DispatchMessageW`
- Handle events: `WM_PAINT`, `WM_CLOSE`, `WM_DESTROY`, `WM_SIZE`
- Blit framebuffer using `StretchDIBits` with `BITMAPINFO` structure
- Resizable window with framebuffer recreation on resize

**API:**
```rust
pub struct Window {
    hwnd: HWND,
    width: u32,
    height: u32,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self>;
    pub fn poll_events(&mut self) -> Vec<Event>;
    pub fn blit_framebuffer(&self, framebuffer: &Framebuffer);
    pub fn is_open(&self) -> bool;
}
```

**Module:** `platform::win32`

**Future:** Architecture supports adding `platform::x11`, `platform::wayland` later.

### Layer 2: Framebuffer

**Responsibility:** Platform-agnostic pixel buffer management.

**Implementation:**
```rust
pub struct Framebuffer {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}
```

**Color Format:** 32-bit RGBA as `u32` in 0xAARRGGBB format (little-endian: BB GG RR AA)

**Operations:**
- `new(width, height)` - allocate buffer, initialize to black
- `clear(color)` - fill entire buffer (SIMD candidate)
- `set_pixel(x, y, color)` - bounds-checked write
- `get_pixel(x, y)` - bounds-checked read
- `as_slice()` - raw buffer access for platform blitting

**Layout:** Row-major: `pixels[y * width + x]`

**Safety:** Bounds checking in debug, potential unchecked variants for proven hot paths.

### Layer 3: Primitives

**Responsibility:** Rasterization algorithms operating on framebuffer.

**Initial Implementation:**

**Pixel Plotting:**
```rust
pub fn plot_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32)
```
Bounds-checked pixel write - foundation for all primitives.

**Bresenham Line Drawing:**
```rust
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32)
```
Integer-only algorithm handling all octants, no floating point or division.

**Future Primitives:**
- Filled triangles (scanline conversion)
- Circles (Bresenham circle algorithm)
- Texture mapping
- Z-buffering

**Module:** `primitives` with potential `primitives::simd` submodule.

### Layer 4: Math

**Responsibility:** Vector and matrix operations for transformations.

**Types:**
```rust
pub struct Vec2 { pub x: f32, pub y: f32 }
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }  // for 3D later
pub struct Mat2 { pub m: [[f32; 2]; 2] }
pub struct Mat4 { pub m: [[f32; 4]; 4] }  // for 3D later
```

**Operations:**
- Vector: add, subtract, scale, dot product, length
- Mat2: rotation matrix creation, transform Vec2
- Mat4: perspective projection, full 3D transforms (future)

**SIMD Strategy:**
- Start with scalar implementations
- Add SIMD variants for batch operations
- Use `std::arch` with `#[cfg(target_feature)]`
- Profile to verify improvements (>20% speedup threshold)

**SIMD Targets:**
- Batch vertex transforms (4 vertices at once)
- Large buffer operations (clear, fill)
- Matrix multiplication (especially Mat4)

**Module:** `math` with potential `math::simd` submodule.

### Layer 5: Application

**Responsibility:** Demo application and game loop.

**Initial Demo:** Rotating wireframe polygon (octagon or square)

**Fixed Timestep Loop:**
```rust
const TARGET_FPS: u64 = 60;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

loop {
    let frame_start = Instant::now();

    // 1. Poll window events
    // 2. Update: increment rotation angle
    // 3. Render:
    //    - Clear framebuffer
    //    - Transform polygon vertices by rotation matrix
    //    - Translate to screen center
    //    - Draw lines between consecutive vertices
    // 4. Blit to window

    let elapsed = frame_start.elapsed();
    if elapsed < FRAME_TIME {
        sleep(FRAME_TIME - elapsed);
    }
}
```

**Rendering Flow:**
1. Vertices defined at origin
2. Apply Mat2 rotation
3. Translate to screen center (width/2, height/2)
4. Convert f32 to i32 coordinates
5. Draw lines connecting vertices (including last→first)

## Testing Strategy

**TDD Approach:** Write tests first, watch them fail, implement minimal code, refactor.

**Unit Tests:**
- Math operations: Vec2 add/sub/scale, Mat2 rotation, transforms
- Framebuffer: creation, clearing, pixel access, bounds checking
- Primitives: pixel plotting, line drawing in all octants
- Color format correctness

**Property-Based Tests:**
- Rotation preserves vector length
- Matrix inverse properties
- Symmetry properties of primitives

**Visual Regression Tests:**
- Render primitives to buffer
- Compare against golden images
- Catch rendering bugs

**Integration Tests:**
- Full rendering pipeline
- Window → framebuffer → primitives → blit

**Test Organization:** `tests/` directory with submodules per layer.

## Benchmarking Strategy

**Framework:** Use `criterion` for reliable, statistical benchmarks.

**Benchmark Targets:**
- Scalar vs SIMD comparisons for each optimization
- Primitive throughput (pixels/sec, lines/sec)
- Full frame rendering time
- Batch operations (1000 vertex transforms, etc.)

**Optimization Criteria:**
- Only add SIMD where benchmarks show >20% improvement
- Identify memory-bound vs compute-bound operations
- Profile before optimizing

**Benchmark Organization:** `benches/` directory with criterion harnesses.

## Development Phases

### Phase 1: Foundation (Initial)
- [ ] Win32 window creation and event loop
- [ ] Framebuffer implementation
- [ ] Pixel plotting
- [ ] Basic Vec2 and Mat2 math
- [ ] Tests for all of the above

### Phase 2: Lines & Rotation
- [ ] Bresenham line algorithm
- [ ] Line drawing tests (all octants)
- [ ] Rotating polygon demo
- [ ] Fixed timestep loop
- [ ] Benchmarks for primitives

### Phase 3: SIMD Optimization
- [ ] Benchmark scalar implementations
- [ ] Add SIMD variants
- [ ] Compare performance
- [ ] Document wins/losses

### Phase 4: More Primitives (Future)
- [ ] Filled triangles
- [ ] Circles
- [ ] Clipping algorithms

### Phase 5: 3D Pipeline (Future)
- [ ] Vec3, Mat4 implementation
- [ ] Perspective projection
- [ ] Z-buffering
- [ ] Rotating 3D wireframe cube
- [ ] Texture mapping

### Phase 6: Vulkan Learning (Future)
- [ ] Parallel Vulkan implementation
- [ ] Compare software vs GPU rendering
- [ ] Understand GPU pipeline mapping

## Success Criteria

**Immediate:**
- Smooth 60 FPS rotating polygon in a window
- Clean, tested, well-documented code
- Benchmarks showing baseline performance

**Medium-term:**
- SIMD optimizations with measurable gains
- 3D wireframe rendering with perspective
- Educational value: deep understanding of graphics pipeline

**Long-term:**
- Foundation for advanced techniques (BSP, texturing, lighting)
- Vulkan implementation for comparison
- Reference implementation for learning graphics programming

## Non-Goals

- Production-ready graphics engine
- Supporting every platform immediately (Win32 first)
- GUI or complex user interaction (focus on rendering)
- Using existing graphics libraries (wgpu, minifb, etc.)

## References

- Michael Abrash's Graphics Programming Black Book
- Bresenham's line algorithm
- Fixed timestep game loops
- Rust SIMD documentation (`std::arch`)
