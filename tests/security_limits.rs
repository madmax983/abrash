use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

const MAX_DIM: u32 = 16384;

#[test]
fn test_framebuffer_oom_prevention() {
    let result = Framebuffer::new(MAX_DIM + 1, 100);
    assert!(result.is_err(), "Allocation should fail for huge width");

    let result = Framebuffer::new(100, MAX_DIM + 1);
    assert!(result.is_err(), "Allocation should fail for huge height");

    let result = Framebuffer::new(MAX_DIM, MAX_DIM);
    assert!(
        result.is_ok(),
        "Allocation should succeed for max allowed size"
    );
}

#[test]
fn test_zbuffer_oom_prevention() {
    let result = ZBuffer::new(MAX_DIM + 1, 100);
    assert!(result.is_err());

    let result = ZBuffer::new(MAX_DIM, MAX_DIM);
    assert!(result.is_ok());
}

#[test]
fn test_texture_oom_prevention() {
    let result = Texture::new(MAX_DIM + 1, 100);
    assert!(result.is_err());

    let result = Texture::new(MAX_DIM, MAX_DIM);
    assert!(result.is_ok());
}
