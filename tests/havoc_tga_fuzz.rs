use abrash::framebuffer::Framebuffer;

#[test]
fn havoc_fuzz_tga_truncation() {
    // TGA headers use 16-bit integers for dimensions (max 65535)
    // Framebuffer allows dimensions up to i32::MAX as long as width * height < u32::MAX
    let width = 70_000;
    let height = 10;

    // This should create a valid framebuffer
    let fb = Framebuffer::new(width, height).unwrap();

    // But exporting it to TGA should return an error, because 70,000 > 65,535
    // If it succeeds, it means it silently truncated the width to `70_000 & 0xFFFF` = 4464
    let result = fb.export_tga("havoc_test_trunc.tga");

    // If it passes, delete the file so we don't pollute the test environment
    if result.is_ok() {
        let _ = std::fs::remove_file("havoc_test_trunc.tga");
    }

    // We expect this to fail, returning an error (e.g. InvalidInput)
    assert!(
        result.is_err(),
        "TGA export succeeded but should have failed due to dimension truncation! Width {width} was truncated."
    );
}
