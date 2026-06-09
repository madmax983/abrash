use abrash_render::render_api::borrowed_target::BorrowedRenderTarget;

#[test]
fn test_borrowed_target_overflow() {
    let mut pixels = vec![0_u32; 1];
    let mut depths = vec![0_f32; 1];
    let result = BorrowedRenderTarget::new(u32::MAX, u32::MAX, &mut pixels, &mut depths);
    assert_eq!(result.err(), Some("buffer dimensions overflow"));
}
