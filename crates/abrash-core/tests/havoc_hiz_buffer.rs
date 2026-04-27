use abrash_core::hiz_buffer::HiZBuffer;

#[test]
#[ignore = "👹 Havoc: Bypass Hi-Z dimensions check causing capacity overflow panic instead of expected custom error"]
#[should_panic(expected = "capacity overflow")]
fn havoc_hiz_capacity_overflow() {
    // 👹 Havoc: The `HiZBuffer::new` check multiplies width * height * 4 and verifies it fits in `usize`.
    // However, Rust's `Vec` maximum capacity is limited to `isize::MAX` bytes.
    // By providing dimensions where `isize::MAX < width * height * 4 < usize::MAX`,
    // we bypass the `expect("Hi-Z dimensions overflow")` check entirely,
    // resulting in an uncontrolled standard library `capacity overflow` panic!
    let w = 4_294_967_295; // u32::MAX
    let h = 536_870_913; // Chosen so w * h * 4 is slightly above isize::MAX but below usize::MAX

    let _ = HiZBuffer::new(w, h);
}
