//! Render target combining pixel and depth buffers.

use crate::framebuffer::Framebuffer;
use crate::hiz_buffer::HiZBuffer;
use crate::render_api::borrowed_target::BorrowedRenderTarget;
use crate::zbuffer::ZBuffer;

/// A render target combining pixel buffer, depth buffer, and optional Hi-Z pyramid.
///
/// This is the primary output surface for the renderer. External consumers create
/// a `RenderTarget` and pass it to [`crate::render_api::cpu_renderer::CpuRenderer::render_frame`].
///
/// # Examples
///
/// ```
/// use abrash_render::render_api::RenderTarget;
///
/// let mut target = RenderTarget::new(800, 600).unwrap();
/// target.clear(0x00FF_000000); // Clear to black
/// assert_eq!(target.width(), 800);
/// assert_eq!(target.height(), 600);
/// ```
pub struct RenderTarget {
    pub(crate) framebuffer: Framebuffer,
    pub(crate) zbuffer: ZBuffer,
    pub(crate) hiz: Option<HiZBuffer>,
}

impl RenderTarget {
    /// Create a new render target with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are invalid (exceed `i32::MAX` or overflow).
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        let framebuffer = Framebuffer::new(width, height)?;
        let zbuffer = ZBuffer::new(width, height)?;
        Ok(Self {
            framebuffer,
            zbuffer,
            hiz: None,
        })
    }

    /// Enable hierarchical z-buffer for occlusion culling.
    pub fn enable_hiz(&mut self) {
        if self.hiz.is_none() {
            self.hiz = Some(HiZBuffer::new(self.width(), self.height()));
        }
    }

    /// Clear both pixel and depth buffers.
    pub fn clear(&mut self, color: u32) {
        self.framebuffer.clear(color);
        self.zbuffer.clear();
    }

    /// Width of the render target in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.framebuffer.width()
    }

    /// Height of the render target in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.framebuffer.height()
    }

    /// Read-only access to the pixel buffer (for display or export).
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        self.framebuffer.as_slice()
    }

    /// Mutable access to the pixel buffer (for post-processing).
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        self.framebuffer.as_mut_slice()
    }

    /// Read-only access to the depth buffer.
    #[must_use]
    pub fn depths(&self) -> &[f32] {
        self.zbuffer.as_slice()
    }

    /// Borrow the underlying buffers as a borrowed render target.
    ///
    /// # Panics
    /// Panics if the internal framebuffer and zbuffer slices do not match the expected
    /// width and height (should be mathematically impossible).
    #[must_use]
    pub fn borrow_mut(&mut self) -> BorrowedRenderTarget<'_> {
        BorrowedRenderTarget::new(
            self.width(),
            self.height(),
            self.framebuffer.as_mut_slice(),
            self.zbuffer.as_mut_slice(),
        )
        .expect("owned render target has matching color and depth buffers")
    }

    /// Split the target into mutable pixel and depth slices.
    pub fn split_mut(&mut self) -> (&mut [u32], &mut [f32]) {
        (self.framebuffer.as_mut_slice(), self.zbuffer.as_mut_slice())
    }

    /// Access the underlying Framebuffer (for compatibility with existing code).
    #[must_use]
    pub const fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    /// Mutable access to the underlying Framebuffer.
    pub const fn framebuffer_mut(&mut self) -> &mut Framebuffer {
        &mut self.framebuffer
    }

    /// Access the underlying `ZBuffer`.
    #[must_use]
    pub const fn zbuffer(&self) -> &ZBuffer {
        &self.zbuffer
    }

    /// Mutable access to the underlying `ZBuffer`.
    pub const fn zbuffer_mut(&mut self) -> &mut ZBuffer {
        &mut self.zbuffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_target_creation() {
        let target = RenderTarget::new(800, 600).unwrap();
        assert_eq!(target.width(), 800);
        assert_eq!(target.height(), 600);
        assert_eq!(target.pixels().len(), 800 * 600);
        assert_eq!(target.depths().len(), 800 * 600);
    }

    #[test]
    fn test_render_target_invalid_dimensions() {
        assert!(RenderTarget::new(i32::MAX as u32 + 1, 600).is_err());
    }

    #[test]
    fn test_render_target_clear() {
        let mut target = RenderTarget::new(10, 10).unwrap();
        target.clear(0xFFFF_0000);
        assert_eq!(target.pixels()[0], 0xFFFF_0000);
        assert!(target.depths()[0].is_infinite());
    }

    #[test]
    fn test_render_target_hiz() {
        let mut target = RenderTarget::new(100, 100).unwrap();
        assert!(target.hiz.is_none());
        target.enable_hiz();
        assert!(target.hiz.is_some());
        // Calling again is a no-op
        target.enable_hiz();
        assert!(target.hiz.is_some());
    }

    #[test]
    fn test_render_target_pixel_access() {
        let mut target = RenderTarget::new(10, 10).unwrap();
        target.pixels_mut()[0] = 0xDEAD_BEEF;
        assert_eq!(target.pixels()[0], 0xDEAD_BEEF);
    }
}
