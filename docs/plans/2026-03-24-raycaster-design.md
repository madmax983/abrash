# Raycaster Design

**Date:** 2026-03-24
**Status:** Approved

## Motivation

Three use cases under one umbrella:

1. **Hybrid rendering** — Raycasted environment geometry + triangle-rasterized objects (sprites, models, particles). The "Doom trick" with a z-buffer bridge.
2. **Optimization testbed** — BAM lookup tables and DDA grid stepping in a context where they genuinely outperform polynomial approximations and floating-point math.
3. **Utility raycasting** — LOS queries, visibility checks, collision detection. The spatial-query role raycasting plays in every modern engine.

## Architecture

```
abrash-raycast → abrash-core (Bam, Fixed16_16, math)
abrash-render  → abrash-core + abrash-raycast
```

- **`abrash-core`** — BAM angle type, 16.16 fixed-point, precomputed sine table (all ported from doom-rs)
- **`abrash-raycast`** — Standalone raycasting crate. Stateless free functions, borrow-only, `Send + Sync` by construction. No framebuffer dependency.
- **`abrash-render`** — Consumes `abrash-raycast` for hybrid rendering (raycasted columns composited with triangle-rasterized geometry via z-buffer)

### Design Decisions

**Port math, rewrite engine** — `Bam` and `Fixed16_16` are ported verbatim from doom-rs (Verus-proved, battle-tested). The raycaster engine is written fresh with a clean API designed for hybrid rendering, LOS, and ECS integration — not inherited from Doom's WAD/Level coupling.

**Layered map traits** — `GridMap` (uniform grid, DDA fast path) and `SectorMap` (arbitrary geometry, BSP traversal). Grid built first; sector designed but deferred. Grid is where BAM + DDA shine and is sufficient for LOS, collision, and the optimization testbed.

**Tiered query API** — Three detail levels sharing the same DDA internals:
- `cast_los` — fastest, early-out, no distance computation
- `cast_ray` — distance + cell + side (collision, gameplay, rendering)
- `cast_ray_detailed` — adds hit point, normal, texture U (rendering only)

**Stateless for ECS** — All functions are free-standing, take borrowed data, hold zero state. Any ECS system can call them from any thread without synchronization. Batching is an optional layer on top with optional rayon parallelism.

**Bam as universal angle, dual trig backends** — `Bam` is the angle representation everywhere (wrapping arithmetic, no clamping). Trig dispatch: table lookup for raycaster hot path (one lookup per ray), `fast_sin_cos` polynomial for post-processing effects (stays in registers, no cache pollution).

**Const sine table** — Unlike doom-rs's current `static mut` runtime-initialized table, the abrash port uses `const` compile-time generation. Same math, no unsafe, no initialization check. (Follow-up: backport this to doom-rs.)

## Core Math (`abrash-core`)

### `bam.rs`

```rust
/// Binary Angle Measure: 2³² = full circle.
/// Wrapping arithmetic — every u32 is a valid angle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bam(pub u32);

// Cardinal constants
pub const ANG45: Bam = Bam(0x2000_0000);
pub const ANG90: Bam = Bam(0x4000_0000);
pub const ANG180: Bam = Bam(0x8000_0000);
pub const ANG270: Bam = Bam(0xC000_0000);

// 8192-entry sine table, const-generated at compile time
const SINE_TABLE: [Fixed16_16; 8192] = generate_sine_table();

impl Bam {
    /// Table lookup — fast path for raycaster (one cache line hit per ray)
    pub fn sin_cos_fixed(&self) -> (Fixed16_16, Fixed16_16);

    /// Delegates to fast_sin_cos polynomial — for effects/one-off use
    pub fn sin_cos_f32(&self) -> (f32, f32);

    // Wrapping arithmetic, from_radians, to_radians, etc.
}
```

### `fixed16_16.rs`

```rust
/// Fixed-point 16.16: bits [31..16] integer, [15..0] fraction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed16_16(pub i32);

impl Fixed16_16 {
    pub fn fixed_mul(self, rhs: Self) -> Self;  // 64-bit intermediate
    pub fn fixed_div(self, rhs: Self) -> Self;
    pub fn to_f32(self) -> f32;
    pub fn from_f32(v: f32) -> Self;
}
```

Coexists with existing 24.8 fixed-point (different precision, different domain — 24.8 for z-buffer, 16.16 for world coordinates).

## Map Traits (`abrash-raycast`)

```rust
/// Uniform grid — DDA + BAM fast path
pub trait GridMap {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn cell_at(&self, x: u32, y: u32) -> Cell;
}

/// Arbitrary geometry — BSP traversal (Phase 5, designed but deferred)
pub trait SectorMap {
    fn walls(&self) -> &[Wall];
    fn sectors(&self) -> &[Sector];
    fn bsp_nodes(&self) -> &[BspNode];
}

pub enum Cell {
    Empty,
    Solid(u16),      // wall material ID
    Portal(u16),     // connected space ID
}
```

Both traits are `Send + Sync` (shared references only). ECS owns the map data, raycaster borrows it.

## Raycasting API (`abrash-raycast`)

### Tiered Queries

```rust
/// Fastest — early-out on first solid hit. AI/ECS hot path.
pub fn cast_los(map: &impl GridMap, from: Vec2<Fixed16_16>, to: Vec2<Fixed16_16>) -> bool

/// Middle tier — distance + what you hit. Collision, gameplay, rendering.
pub fn cast_ray(map: &impl GridMap, origin: Vec2<Fixed16_16>, angle: Bam) -> Option<RayHit>

/// Full detail — rendering path only.
pub fn cast_ray_detailed(map: &impl GridMap, origin: Vec2<Fixed16_16>, angle: Bam) -> Option<DetailedHit>
```

### Return Types

```rust
pub struct RayHit {
    pub distance: Fixed16_16,    // perpendicular (no fisheye)
    pub cell_x: u32,
    pub cell_y: u32,
    pub side: Side,
}

pub struct DetailedHit {
    pub hit: RayHit,
    pub point: Vec2<Fixed16_16>, // exact world-space hit
    pub normal: Vec2<Fixed16_16>,
    pub texture_u: Fixed16_16,   // 0..1 along wall face
}

pub enum Side { North, South, East, West }
```

### Batching

```rust
/// Scatter-gather for ECS systems. Optional rayon parallelism.
pub fn cast_rays_batch(
    map: &impl GridMap,
    rays: &[(Vec2<Fixed16_16>, Bam)],
    results: &mut [Option<RayHit>],
)

pub fn cast_los_batch(
    map: &impl GridMap,
    pairs: &[(Vec2<Fixed16_16>, Vec2<Fixed16_16>)],
    results: &mut [bool],
)
```

## DDA Engine (`abrash-raycast::dda`)

```rust
/// Internal iterator — steps through grid cells along a ray
struct DdaStepper {
    cell_x: i32,
    cell_y: i32,
    step_x: i32,           // +1 or -1
    step_y: i32,
    side_dist_x: Fixed16_16,
    side_dist_y: Fixed16_16,
    delta_dist_x: Fixed16_16,
    delta_dist_y: Fixed16_16,
    last_side: Side,
}
```

Trig used **once** at ray setup (`Bam::sin_cos_fixed()` table lookup) to compute delta distances. Inner loop is pure fixed-point add + compare — no trig, no sqrt.

Perpendicular distance (fisheye correction) is free from the DDA structure:

```rust
let perp_dist = match last_side {
    EastWest => side_dist_x - delta_dist_x,
    NorthSouth => side_dist_y - delta_dist_y,
};
```

## Hybrid Rendering (`abrash-render`)

```rust
/// Raycasted columns to framebuffer
pub fn render_raycast_view(
    fb: &mut Framebuffer,
    map: &impl GridMap,
    camera_pos: Vec2<Fixed16_16>,
    camera_angle: Bam,
    fov: Bam,
)

/// Full hybrid: raycasted walls + triangle-rasterized objects
pub fn render_hybrid(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    map: &impl GridMap,
    camera: &Camera,
    meshes: &[RenderMesh],
)
```

Z-buffer bridge: raycaster writes perpendicular distances per column, triangle rasterizer depth-tests against them. Objects correctly occlude behind walls.

## Crate Layout

```
abrash/
├── crates/
│   ├── abrash-core/src/
│   │   ├── math.rs              (existing — fast_sin_cos stays)
│   │   ├── bam.rs               (NEW)
│   │   ├── fixed16_16.rs        (NEW)
│   │   └── ...
│   ├── abrash-raycast/          (NEW CRATE)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── map.rs           (GridMap, SectorMap traits)
│   │       ├── cast.rs          (cast_los, cast_ray, cast_ray_detailed)
│   │       ├── dda.rs           (DdaStepper)
│   │       ├── batch.rs         (scatter-gather, optional rayon)
│   │       └── types.rs         (RayHit, DetailedHit, Side, Cell, Wall)
│   ├── abrash-render/src/
│   │   ├── raycaster/           (NEW MODULE)
│   │   │   ├── mod.rs
│   │   │   └── hybrid.rs
│   │   └── ...
│   └── ...
├── benches/
│   └── raycast_bench.rs         (NEW)
└── examples/
    ├── raycast_demo.rs          (NEW)
    └── hybrid_demo.rs           (NEW)
```

**Feature flags on `abrash-raycast`:**
- `parallel` — rayon for batch operations
- `sector-map` — SectorMap/BSP path (off by default)

## Testing Strategy

**`abrash-core` (Bam/Fixed16_16):**
- Roundtrip: `Bam::from_radians(x).to_radians() ≈ x`
- Wrapping: `ANG180 + ANG180 == ZERO`
- Table accuracy: lookup vs `f32::sin()`, error < 0.001
- Fixed16_16 mul/div overflow (Verus proofs from doom-rs)
- Const table == runtime computation

**`abrash-raycast` (DDA/casting):**
- Axis-aligned rays: known cell at known distance
- Diagonal rays: compare against analytical solution
- `cast_los`: true through corridor, false through wall
- `cast_los` symmetry: `cast_los(a, b) == cast_los(b, a)`
- Edge cases: origin on grid boundary, ray parallel to wall, zero-length LOS
- Property tests (proptest): random origin + angle, distance ≥ 0, hit cell is solid

**Benchmarks (criterion):**
- Single `cast_ray`: map sizes 16×16 through 256×256
- `cast_los_batch`: 100 / 1,000 / 10,000 queries
- BAM table vs `fast_sin_cos` vs `f32::sin_cos` — per-ray setup cost
- DDA steps/second: sparse vs maze density

## Build Order

| Phase | Scope | Deliverable |
|-------|-------|-------------|
| 1 | `Bam` + `Fixed16_16` in `abrash-core` | Ported math, tests, proofs, trig benchmark |
| 2 | `abrash-raycast` grid path | `GridMap`, DDA, tiered queries, test suite, perf benchmarks |
| 3 | Batch + parallel | `cast_rays_batch`, `cast_los_batch`, rayon feature, scaling benchmarks |
| 4 | Hybrid rendering in `abrash-render` | Raycasted columns, z-buffer bridge, demo examples |
| 5 | `SectorMap` / BSP (future) | Deferred until a consumer needs arbitrary geometry |

## Follow-ups

- **doom-rs**: Convert `SINE_TABLE` from `static mut` runtime init to `const` compile-time generation
