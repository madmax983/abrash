use abrash::clipping::{clip_triangle_to_frustum, Lerp};
use abrash::math::Vec3;

#[derive(Clone, Copy, Debug)]
struct Bomb {
    ptr: *const u8,
}

impl Lerp for Bomb {
    fn lerp(self, _other: Self, _t: f32) -> Self {
        self
    }
}

unsafe impl Send for Bomb {}
unsafe impl Sync for Bomb {}

#[test]
#[should_panic(expected = "out of bounds")]
fn test_havoc_clipping_ub() {
    // A pointer to a valid byte
    let valid_byte = 42u8;
    let valid_ptr = &valid_byte as *const u8;

    let v0 = Bomb { ptr: valid_ptr };
    let v1 = Bomb { ptr: valid_ptr };
    let v2 = Bomb { ptr: valid_ptr };

    // Function to get position - returns points far outside frustum to ensure culling
    // w=1.0, x=100.0 -> x > w -> culled by right plane
    let get_pos = |_b: &Bomb| (Vec3::new(100.0, 0.0, 0.0), 1.0);

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    // Result count should be 0
    assert_eq!(result.count, 0);

    // Accessing index 0.
    // In release mode with debug_assert!, this bypasses the check.
    // It returns a reference to uninitialized memory (MaybeUninit).
    // The memory might be zeroed or garbage.
    // If it's zero/null, dereferencing it segfaults.
    // If it's garbage, it segfaults.
    // If we are lucky and it's valid_ptr, it doesn't crash (false negative), but it's still UB.

    // This line reads uninitialized memory to create a copy of Bomb.
    let bomb = result[0];
    unsafe {
        // This dereference should segfault if ptr is garbage/null.
        println!("Bomb value: {}", *bomb.ptr);
    }
}
