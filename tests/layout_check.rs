#![allow(clippy::float_cmp)]

use abrash::geometry::BoundingSphere;
use abrash::math::{Mat4, Vec2, Vec3, Vec4};
use std::mem;

#[test]
fn test_vec2_layout() {
    // x, y
    assert_eq!(mem::size_of::<Vec2>(), 8);
    assert_eq!(mem::align_of::<Vec2>(), 4);
    let v = Vec2 { x: 1.0, y: 2.0 };
    let ptr = (&raw const v).cast::<f32>();
    unsafe {
        assert_eq!(*ptr.add(0), 1.0);
        assert_eq!(*ptr.add(1), 2.0);
    }
}

#[test]
fn test_vec3_layout() {
    // x, y, z
    assert_eq!(mem::size_of::<Vec3>(), 12);
    assert_eq!(mem::align_of::<Vec3>(), 4);
    let v = Vec3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let ptr = (&raw const v).cast::<f32>();
    unsafe {
        assert_eq!(*ptr.add(0), 1.0);
        assert_eq!(*ptr.add(1), 2.0);
        assert_eq!(*ptr.add(2), 3.0);
    }
}

#[test]
fn test_vec4_layout() {
    // x, y, z, w
    assert_eq!(mem::size_of::<Vec4>(), 16);
    assert_eq!(mem::align_of::<Vec4>(), 4);
    let v = Vec4 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        w: 4.0,
    };
    let ptr = (&raw const v).cast::<f32>();
    unsafe {
        assert_eq!(*ptr.add(0), 1.0);
        assert_eq!(*ptr.add(1), 2.0);
        assert_eq!(*ptr.add(2), 3.0);
        assert_eq!(*ptr.add(3), 4.0);
    }
}

#[test]
fn test_mat4_layout() {
    // 16 floats, row major
    assert_eq!(mem::size_of::<Mat4>(), 64);
    // Mat4 is 16-byte aligned for SIMD
    assert_eq!(mem::align_of::<Mat4>(), 16);

    let m = Mat4 {
        m: [
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
            [12.0, 13.0, 14.0, 15.0],
        ],
    };
    let ptr = (&raw const m).cast::<f32>();
    unsafe {
        for i in 0..16 {
            assert_eq!(*ptr.add(i), i as f32);
        }
    }
}

#[test]
fn test_bounding_sphere_layout() {
    // x, y, z, r
    // BoundingSphere is #[repr(C)] { Vec3, f32 }
    // Size should be 12 + 4 = 16.
    assert_eq!(mem::size_of::<BoundingSphere>(), 16);
    assert_eq!(mem::align_of::<BoundingSphere>(), 4);

    let s = BoundingSphere {
        center: Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        radius: 4.0,
    };
    let ptr = (&raw const s).cast::<f32>();

    // Critical assertion: The memory layout MUST be x, y, z, r packed.
    // The AVX culling code depends on this to load 8 spheres into vector registers.
    unsafe {
        assert_eq!(*ptr.add(0), 1.0); // x
        assert_eq!(*ptr.add(1), 2.0); // y
        assert_eq!(*ptr.add(2), 3.0); // z
        assert_eq!(*ptr.add(3), 4.0); // r
    }
}
