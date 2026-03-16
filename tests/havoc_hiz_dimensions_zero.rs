use abrash::hiz_buffer::HiZBuffer;

#[test]
fn havoc_hiz_buffer_dimensions_zero() {
    let _hiz = HiZBuffer::new(0, 0);
}
