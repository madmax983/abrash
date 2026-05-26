use abrash_core::zbuffer::ZBuffer;

#[test]
#[ignore = "👹 Havoc: Bypass ZBuffer bounds check causing severe memory violation and SIGSEGV"]
fn test_havoc_zbuffer_panic() {
    let mut zb = ZBuffer::new(100, 100).unwrap();
    // Intentionally pass OOB to cause crash.
    unsafe {
        zb.test_and_set_unchecked(1000, 1000, 0.5);
    }
}
