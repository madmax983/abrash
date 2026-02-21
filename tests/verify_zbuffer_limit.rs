use abrash::zbuffer::ZBuffer;

#[test]
fn test_zbuffer_rejects_huge_allocation() {
    // 46341 * 46341 = 2,147,488,281
    // i32::MAX      = 2,147,483,647
    // Difference    = 4,634
    // This size fits in u32 but exceeds i32::MAX.
    // If accepted, it causes integer overflow in SIMD gather operations which use signed 32-bit offsets.

    let width = 46341;
    let height = 46341;

    // We expect this to fail gracefully with an Err, not panic (OOM) or succeed.
    // Note: If the fix is not applied, this might panic with OOM on systems with <8GB RAM,
    // or return Ok if memory is available.
    // Ideally, we want to enforce the limit regardless of available RAM.

    match ZBuffer::new(width, height) {
        Ok(_) => panic!("ZBuffer should have rejected size > i32::MAX"),
        // Expect the size overflow error, not the dimension limit error
        Err(e) => assert_eq!(e, "Buffer size overflow (max i32::MAX pixels)"),
    }
}
