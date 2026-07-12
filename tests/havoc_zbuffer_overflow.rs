use abrash_core::zbuffer::ZBuffer;

#[test]
#[ignore = "👺 Havoc: ZBuffer Integer Overflow causes Out-Of-Bounds Memory Access"]
fn test_zbuffer_overflow_exploit() {
    let mut zb = ZBuffer::new(10, 10).unwrap();

    // Trigger an overflow in clear_rect calculation
    // zb.clear_rect takes (x: i32, y: i32, width: u32, height: u32)
    // If x = -5, y = -5, width = u32::MAX, height = u32::MAX
    // x2_i64 = (-5) + u32::MAX (which is 4294967295) = 4294967290
    zb.clear_rect(-5, -5, u32::MAX, u32::MAX);

    // Check if we crashed during calculation
}
