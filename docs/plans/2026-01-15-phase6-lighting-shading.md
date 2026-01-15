# Phase 6: Lighting and Shading

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement flat and Gouraud shading with directional lighting for realistic 3D rendering.

**Architecture:** Add face and vertex normals to meshes, create a lighting system with directional lights, implement flat shading (per-face color) and Gouraud shading (per-vertex color interpolation).

**Tech Stack:** Rust (edition 2024), existing abrash crate with Vec3/Mat4/ZBuffer

---

## Task 1: Face Normals

**Files:**
- Modify: `src/mesh.rs`
- Modify: `tests/mesh_tests.rs` (create if needed)

**Step 1: Write failing test**

Create `tests/mesh_tests.rs`:

```rust
use abrash::mesh::Mesh;
use abrash::math::Vec3;

#[test]
fn test_cube_face_normals() {
    let cube = Mesh::cube(2.0);
    let normals = cube.compute_face_normals();

    // 12 triangles = 12 normals
    assert_eq!(normals.len(), 12);

    // All normals should be unit length
    for n in &normals {
        let len = n.length();
        assert!((len - 1.0).abs() < 0.001, "Normal not unit length: {}", len);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_cube_face_normals`
Expected: FAIL with "compute_face_normals not found"

**Step 3: Implement face normals**

Add to `src/mesh.rs`:

```rust
impl Mesh {
    /// Compute face normal for each triangle
    pub fn compute_face_normals(&self) -> Vec<Vec3> {
        self.indices
            .iter()
            .map(|[i0, i1, i2]| {
                let v0 = self.vertices[*i0];
                let v1 = self.vertices[*i1];
                let v2 = self.vertices[*i2];

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                edge1.cross(edge2).normalize()
            })
            .collect()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_cube_face_normals`
Expected: PASS

**Step 5: Commit**

```bash
git add src/mesh.rs tests/mesh_tests.rs
git commit -m "feat(mesh): add face normal computation

Compute per-face normals for flat shading using cross product.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Light Structure

**Files:**
- Create: `src/light.rs`
- Modify: `src/lib.rs`
- Create: `tests/light_tests.rs`

**Step 1: Write failing test**

Create `tests/light_tests.rs`:

```rust
use abrash::light::DirectionalLight;
use abrash::math::Vec3;

#[test]
fn test_directional_light_intensity() {
    let light = DirectionalLight::new(
        Vec3::new(0.0, -1.0, 0.0), // Light pointing down
        Vec3::new(1.0, 1.0, 1.0),  // White light
    );

    // Surface facing up should be fully lit
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let intensity = light.intensity(normal);
    assert!((intensity - 1.0).abs() < 0.001);

    // Surface facing down should be unlit
    let normal = Vec3::new(0.0, -1.0, 0.0);
    let intensity = light.intensity(normal);
    assert!(intensity < 0.001);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_directional_light_intensity`
Expected: FAIL with "unresolved import"

**Step 3: Implement light module**

Create `src/light.rs`:

```rust
//! Lighting calculations for 3D rendering.

use crate::math::Vec3;

/// Directional light (like sunlight)
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Direction the light travels (normalized)
    pub direction: Vec3,
    /// Light color (RGB, 0.0-1.0)
    pub color: Vec3,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: Vec3) -> Self {
        Self {
            direction: direction.normalize(),
            color,
        }
    }

    /// Calculate light intensity on a surface with given normal
    /// Returns 0.0-1.0 based on Lambert's cosine law
    pub fn intensity(&self, normal: Vec3) -> f32 {
        // N dot L (light direction is inverted because it points AT the surface)
        let n_dot_l = normal.dot(self.direction * -1.0);
        n_dot_l.max(0.0)
    }

    /// Calculate lit color for a surface
    pub fn shade(&self, normal: Vec3, base_color: Vec3) -> Vec3 {
        let i = self.intensity(normal);
        Vec3::new(
            base_color.x * self.color.x * i,
            base_color.y * self.color.y * i,
            base_color.z * self.color.z * i,
        )
    }
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Convert u32 ARGB to Vec3 color (0.0-1.0 per channel)
pub fn u32_to_color(argb: u32) -> Vec3 {
    let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
    let b = (argb & 0xFF) as f32 / 255.0;
    Vec3::new(r, g, b)
}
```

**Step 4: Add module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod light;
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_directional_light_intensity`
Expected: PASS

**Step 6: Add color conversion tests**

Add to `tests/light_tests.rs`:

```rust
use abrash::light::{color_to_u32, u32_to_color};

#[test]
fn test_color_conversion_roundtrip() {
    let original = 0xFFFF8040; // Orange
    let color = u32_to_color(original);
    let back = color_to_u32(color);
    assert_eq!(back, original);
}

#[test]
fn test_color_to_u32() {
    let white = Vec3::new(1.0, 1.0, 1.0);
    assert_eq!(color_to_u32(white), 0xFFFFFFFF);

    let red = Vec3::new(1.0, 0.0, 0.0);
    assert_eq!(color_to_u32(red), 0xFFFF0000);
}
```

**Step 7: Run all light tests**

Run: `cargo test light`
Expected: PASS (3 tests)

**Step 8: Commit**

```bash
git add src/light.rs src/lib.rs tests/light_tests.rs
git commit -m "feat(light): add directional lighting system

Directional light with Lambert shading and color conversion utilities.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Ambient Lighting

**Files:**
- Modify: `src/light.rs`
- Modify: `tests/light_tests.rs`

**Step 1: Write failing test**

Add to `tests/light_tests.rs`:

```rust
use abrash::light::AmbientLight;

#[test]
fn test_ambient_light() {
    let ambient = AmbientLight::new(Vec3::new(0.1, 0.1, 0.1));
    let base = Vec3::new(1.0, 0.0, 0.0); // Red
    let result = ambient.shade(base);

    assert!((result.x - 0.1).abs() < 0.001);
    assert!(result.y < 0.001);
    assert!(result.z < 0.001);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_ambient_light`
Expected: FAIL with "AmbientLight not found"

**Step 3: Implement ambient light**

Add to `src/light.rs`:

```rust
/// Ambient light (constant illumination)
#[derive(Debug, Clone, Copy)]
pub struct AmbientLight {
    pub color: Vec3,
}

impl AmbientLight {
    pub fn new(color: Vec3) -> Self {
        Self { color }
    }

    /// Apply ambient lighting to a base color
    pub fn shade(&self, base_color: Vec3) -> Vec3 {
        Vec3::new(
            base_color.x * self.color.x,
            base_color.y * self.color.y,
            base_color.z * self.color.z,
        )
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_ambient_light`
Expected: PASS

**Step 5: Commit**

```bash
git add src/light.rs tests/light_tests.rs
git commit -m "feat(light): add ambient lighting

Constant ambient illumination to prevent fully black shadows.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Flat Shading Renderer

**Files:**
- Modify: `src/primitives.rs`
- Modify: `tests/primitives_tests.rs`

**Step 1: Write failing test**

Add to `tests/primitives_tests.rs`:

```rust
use abrash::primitives::fill_triangle_flat;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

#[test]
fn test_fill_triangle_flat_basic() {
    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

    // Create a triangle facing the camera
    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0); // Facing camera
    let color = Vec3::new(1.0, 0.0, 0.0); // Red

    fill_triangle_flat(&mut fb, &mut zb, v0, v1, v2, normal, color);

    // Center should have the shaded color
    let pixel = fb.get_pixel(50, 50);
    assert!(pixel.is_some());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_fill_triangle_flat_basic`
Expected: FAIL with "fill_triangle_flat not found"

**Step 3: Implement flat shaded triangle**

Add to `src/primitives.rs` (add import at top):

```rust
use crate::light::{color_to_u32, DirectionalLight, AmbientLight};
```

Add the function:

```rust
/// Fill a 3D triangle with flat shading
pub fn fill_triangle_flat(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
) {
    // Default lighting setup
    let ambient = AmbientLight::new(Vec3::new(0.2, 0.2, 0.2));
    let sun = DirectionalLight::new(Vec3::new(-0.5, -1.0, -0.5), Vec3::new(1.0, 1.0, 1.0));

    // Calculate flat shade
    let ambient_color = ambient.shade(base_color);
    let diffuse_color = sun.shade(normal, base_color);

    let final_color = Vec3::new(
        (ambient_color.x + diffuse_color.x).min(1.0),
        (ambient_color.y + diffuse_color.y).min(1.0),
        (ambient_color.z + diffuse_color.z).min(1.0),
    );

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

/// Fill a 3D triangle with custom lighting
pub fn fill_triangle_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
    ambient: &AmbientLight,
    light: &DirectionalLight,
) {
    let ambient_color = ambient.shade(base_color);
    let diffuse_color = light.shade(normal, base_color);

    let final_color = Vec3::new(
        (ambient_color.x + diffuse_color.x).min(1.0),
        (ambient_color.y + diffuse_color.y).min(1.0),
        (ambient_color.z + diffuse_color.z).min(1.0),
    );

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_fill_triangle_flat_basic`
Expected: PASS

**Step 5: Commit**

```bash
git add src/primitives.rs tests/primitives_tests.rs
git commit -m "feat(primitives): add flat shading triangle renderer

Per-face lighting with ambient + diffuse using Lambert model.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Vertex Normals

**Files:**
- Modify: `src/mesh.rs`
- Modify: `tests/mesh_tests.rs`

**Step 1: Write failing test**

Add to `tests/mesh_tests.rs`:

```rust
#[test]
fn test_cube_vertex_normals() {
    let cube = Mesh::cube(2.0);
    let normals = cube.compute_vertex_normals();

    // 8 vertices = 8 normals
    assert_eq!(normals.len(), 8);

    // All normals should be unit length
    for n in &normals {
        let len = n.length();
        assert!((len - 1.0).abs() < 0.001, "Normal not unit length: {}", len);
    }

    // Corner vertex normal should point diagonally outward
    // Vertex 0 is at (-h, -h, h), normal should point roughly (-1, -1, 1) normalized
    let n0 = normals[0];
    assert!(n0.x < 0.0);
    assert!(n0.y < 0.0);
    assert!(n0.z > 0.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_cube_vertex_normals`
Expected: FAIL with "compute_vertex_normals not found"

**Step 3: Implement vertex normals**

Add to `src/mesh.rs`:

```rust
impl Mesh {
    /// Compute smooth vertex normals by averaging adjacent face normals
    pub fn compute_vertex_normals(&self) -> Vec<Vec3> {
        let face_normals = self.compute_face_normals();
        let mut vertex_normals = vec![Vec3::zero(); self.vertices.len()];

        // Accumulate face normals at each vertex
        for (face_idx, [i0, i1, i2]) in self.indices.iter().enumerate() {
            let normal = face_normals[face_idx];
            vertex_normals[*i0] = vertex_normals[*i0] + normal;
            vertex_normals[*i1] = vertex_normals[*i1] + normal;
            vertex_normals[*i2] = vertex_normals[*i2] + normal;
        }

        // Normalize each vertex normal
        vertex_normals.iter().map(|n| n.normalize()).collect()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_cube_vertex_normals`
Expected: PASS

**Step 5: Commit**

```bash
git add src/mesh.rs tests/mesh_tests.rs
git commit -m "feat(mesh): add vertex normal computation

Average face normals at vertices for smooth Gouraud shading.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Gouraud Shading

**Files:**
- Modify: `src/primitives.rs`
- Modify: `tests/primitives_tests.rs`

**Step 1: Write failing test**

Add to `tests/primitives_tests.rs`:

```rust
use abrash::primitives::fill_triangle_gouraud;

#[test]
fn test_fill_triangle_gouraud_basic() {
    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

    // Triangle with different colors at each vertex
    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let c0 = Vec3::new(1.0, 0.0, 0.0); // Red
    let c1 = Vec3::new(0.0, 1.0, 0.0); // Green
    let c2 = Vec3::new(0.0, 0.0, 1.0); // Blue

    fill_triangle_gouraud(&mut fb, &mut zb, (v0, c0), (v1, c1), (v2, c2));

    // Should render without panicking
    let pixel = fb.get_pixel(50, 50);
    assert!(pixel.is_some());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_fill_triangle_gouraud_basic`
Expected: FAIL with "fill_triangle_gouraud not found"

**Step 3: Implement Gouraud shading**

Add to `src/primitives.rs`:

```rust
/// Fill a 3D triangle with Gouraud (per-vertex) shading
/// Each vertex has a position (clip space + w) and color
pub fn fill_triangle_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3), // ((position, w), color)
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
) {
    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let (x0, y0, z0) = project_to_screen(v0.0.0, v0.0.1, width, height);
    let (x1, y1, z1) = project_to_screen(v1.0.0, v1.0.1, width, height);
    let (x2, y2, z2) = project_to_screen(v2.0.0, v2.0.1, width, height);

    let c0 = v0.1;
    let c1 = v1.1;
    let c2 = v2.1;

    // Sort by y (bubble sort 3 elements)
    let mut verts = [
        (x0, y0, z0, c0),
        (x1, y1, z1, c1),
        (x2, y2, z2, c2),
    ];
    if verts[0].1 > verts[1].1 { verts.swap(0, 1); }
    if verts[0].1 > verts[2].1 { verts.swap(0, 2); }
    if verts[1].1 > verts[2].1 { verts.swap(1, 2); }

    let (x0, y0, z0, c0) = verts[0];
    let (x1, y1, z1, c1) = verts[1];
    let (x2, y2, z2, c2) = verts[2];

    let total_height = y2 - y0;
    if total_height == 0 {
        return;
    }

    for y in y0..=y2 {
        let second_half = y > y1 || y1 == y0;
        let segment_height = if second_half { y2 - y1 } else { y1 - y0 };
        if segment_height == 0 {
            continue;
        }

        let alpha = (y - y0) as f32 / total_height as f32;
        let beta = if second_half {
            (y - y1) as f32 / segment_height as f32
        } else {
            (y - y0) as f32 / segment_height as f32
        };

        // Interpolate position and color along edges
        let mut ax = x0 as f32 + (x2 - x0) as f32 * alpha;
        let mut az = z0 + (z2 - z0) * alpha;
        let mut ac = Vec3::new(
            c0.x + (c2.x - c0.x) * alpha,
            c0.y + (c2.y - c0.y) * alpha,
            c0.z + (c2.z - c0.z) * alpha,
        );

        let (mut bx, mut bz, mut bc) = if second_half {
            (
                x1 as f32 + (x2 - x1) as f32 * beta,
                z1 + (z2 - z1) * beta,
                Vec3::new(
                    c1.x + (c2.x - c1.x) * beta,
                    c1.y + (c2.y - c1.y) * beta,
                    c1.z + (c2.z - c1.z) * beta,
                ),
            )
        } else {
            (
                x0 as f32 + (x1 - x0) as f32 * beta,
                z0 + (z1 - z0) * beta,
                Vec3::new(
                    c0.x + (c1.x - c0.x) * beta,
                    c0.y + (c1.y - c0.y) * beta,
                    c0.z + (c1.z - c0.z) * beta,
                ),
            )
        };

        if ax > bx {
            std::mem::swap(&mut ax, &mut bx);
            std::mem::swap(&mut az, &mut bz);
            std::mem::swap(&mut ac, &mut bc);
        }

        let x_start = ax as i32;
        let x_end = bx as i32;

        for x in x_start..=x_end {
            let t = if (x_end - x_start) > 0 {
                (x - x_start) as f32 / (x_end - x_start) as f32
            } else {
                0.0
            };

            let z = az + (bz - az) * t;
            let color = Vec3::new(
                ac.x + (bc.x - ac.x) * t,
                ac.y + (bc.y - ac.y) * t,
                ac.z + (bc.z - ac.z) * t,
            );

            if zb.test_and_set(x, y, z) {
                fb.set_pixel(x, y, color_to_u32(color));
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_fill_triangle_gouraud_basic`
Expected: PASS

**Step 5: Commit**

```bash
git add src/primitives.rs tests/primitives_tests.rs
git commit -m "feat(primitives): add Gouraud shading

Per-vertex color interpolation across scanlines for smooth shading.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Transform Normals

**Files:**
- Modify: `src/math.rs`
- Modify: `tests/math_tests.rs`

**Step 1: Write failing test**

Add to `tests/math_tests.rs`:

```rust
#[test]
fn test_mat4_transform_normal() {
    let m = Mat4::rotation_y(std::f32::consts::PI / 2.0); // 90 degree Y rotation
    let normal = Vec3::new(0.0, 0.0, 1.0); // Pointing +Z

    let result = m.transform_normal(normal);

    // After 90 degree Y rotation, +Z becomes +X
    assert!((result.x - 1.0).abs() < 0.01);
    assert!(result.y.abs() < 0.01);
    assert!(result.z.abs() < 0.01);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_mat4_transform_normal`
Expected: FAIL with "transform_normal not found"

**Step 3: Implement normal transformation**

Add to `src/math.rs` in `impl Mat4`:

```rust
    /// Transform a normal vector (ignores translation, uses upper-left 3x3)
    pub fn transform_normal(&self, n: Vec3) -> Vec3 {
        let x = self.m[0][0] * n.x + self.m[1][0] * n.y + self.m[2][0] * n.z;
        let y = self.m[0][1] * n.x + self.m[1][1] * n.y + self.m[2][1] * n.z;
        let z = self.m[0][2] * n.x + self.m[1][2] * n.y + self.m[2][2] * n.z;
        Vec3::new(x, y, z).normalize()
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_mat4_transform_normal`
Expected: PASS

**Step 5: Commit**

```bash
git add src/math.rs tests/math_tests.rs
git commit -m "feat(math): add normal transformation

Transform normals using rotation portion of matrix (no translation).

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Lit Cube Demo

**Files:**
- Modify: `src/main.rs`

**Step 1: Update demo to use flat shading**

Replace `src/main.rs`:

```rust
//! Abrash Graphics Demo - Lit 3D Cube
//!
//! Demonstrates flat shading with directional lighting.

use abrash::framebuffer::Framebuffer;
use abrash::light::{color_to_u32, u32_to_color, AmbientLight, DirectionalLight};
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::Window;
use abrash::primitives::fill_triangle_lit;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Face colors for the cube
const FACE_COLORS: [u32; 6] = [
    0xFFE74C3C, // Red
    0xFF2ECC71, // Green
    0xFF3498DB, // Blue
    0xFFF39C12, // Orange
    0xFF9B59B6, // Purple
    0xFF1ABC9C, // Teal
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Lit Cube", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT);
    let cube = Mesh::cube(1.5);
    let face_normals = cube.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 2.0, 4.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    // Lighting setup
    let ambient = AmbientLight::new(Vec3::new(0.15, 0.15, 0.15));
    let sun = DirectionalLight::new(
        Vec3::new(-0.5, -1.0, -0.3),
        Vec3::new(1.0, 0.95, 0.9), // Warm sunlight
    );

    let mut timestep = FixedTimestep::new(60);
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    while window.is_open() {
        window.poll_events();

        while timestep.should_update() {
            angle_y += 0.02;
            angle_x += 0.008;
        }

        // Clear buffers
        framebuffer.clear(0xFF1A1A2E); // Dark blue background
        zbuffer.clear();

        // Build model matrix
        let model = Mat4::rotation_y(angle_y).mul(&Mat4::rotation_x(angle_x));
        let mvp = projection.mul(&view.mul(&model));

        // Render each face
        for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri_indices;

            // Transform vertices
            let v0 = mvp.transform_point(cube.vertices[i0]);
            let v1 = mvp.transform_point(cube.vertices[i1]);
            let v2 = mvp.transform_point(cube.vertices[i2]);

            // Transform normal to world space (use model matrix only)
            let world_normal = model.transform_normal(face_normals[face_idx]);

            // Get face color (2 triangles per face)
            let face_color = u32_to_color(FACE_COLORS[face_idx / 2]);

            // Render with lighting
            fill_triangle_lit(
                &mut framebuffer,
                &mut zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                face_color,
                &ambient,
                &sun,
            );
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
```

**Step 2: Build and run**

Run: `cargo build --release && cargo run --release`
Expected: Window showing rotating cube with lighting (faces brighten/darken based on angle to light)

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: update demo to lit rotating cube

Flat shaded cube with ambient + directional lighting.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Gouraud Shaded Demo (Optional Enhancement)

**Files:**
- Modify: `src/main.rs`

**Step 1: Add Gouraud shading option**

Add after the flat-shaded cube rendering, or create a second demo mode:

```rust
// For smooth shading, compute vertex colors based on lighting
fn compute_vertex_color(
    normal: Vec3,
    base_color: Vec3,
    ambient: &AmbientLight,
    light: &DirectionalLight,
) -> Vec3 {
    let ambient_color = ambient.shade(base_color);
    let diffuse_color = light.shade(normal, base_color);

    Vec3::new(
        (ambient_color.x + diffuse_color.x).min(1.0),
        (ambient_color.y + diffuse_color.y).min(1.0),
        (ambient_color.z + diffuse_color.z).min(1.0),
    )
}
```

**Step 2: Commit if implemented**

```bash
git add src/main.rs
git commit -m "feat: add Gouraud shading to demo

Smooth per-vertex lighting for organic shapes.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 10: Run Full Test Suite

**Verification step**

```bash
cargo test --all
cargo clippy
cargo build --release
cargo run --release
```

Expected: All tests pass, no warnings, lit cube demo runs

---

## Completion Checklist

- [ ] Face normals (cross product)
- [ ] DirectionalLight with Lambert shading
- [ ] AmbientLight
- [ ] color_to_u32 / u32_to_color utilities
- [ ] fill_triangle_flat with built-in lighting
- [ ] fill_triangle_lit with custom lighting
- [ ] Vertex normals (averaged face normals)
- [ ] fill_triangle_gouraud (color interpolation)
- [ ] Mat4::transform_normal
- [ ] Lit cube demo
- [ ] All tests passing

## Next Steps

After Phase 6:
- **Phase 7:** Texture mapping (UV coordinates, texture sampling)
- **Phase 8:** OBJ file loading, more complex meshes
- **Phase 9:** Specular highlights (Phong shading)
