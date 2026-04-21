//! A render target that wraps externally-owned memory buffers.
//!
//! Unlike [`crate::render_api::RenderTarget`] which allocates and owns its internal memory,
//! a [`BorrowedRenderTarget`] allows you to render directly into existing slices.
//! This is extremely useful for zero-copy integration with platform-specific windowing systems
//! (like softbuffer, minifb, or winit) where the window buffer is provided to you.
//!
//! # Examples
//!
//! ```
//! use abrash_render::render_api::BorrowedRenderTarget;
//!
//! // Imagine these buffers come from a windowing system or FFI boundary
//! let mut color_buffer = vec![0_u32; 800 * 600];
//! let mut depth_buffer = vec![f32::INFINITY; 800 * 600];
//!
//! // Wrap them safely for the renderer
//! let target = BorrowedRenderTarget::new(
//!     800,
//!     600,
//!     &mut color_buffer,
//!     &mut depth_buffer
//! ).expect("Buffers matched dimensions");
//!
//! assert_eq!(target.width(), 800);
//! assert_eq!(target.height(), 600);
//! ```

/// A render target backed by caller-owned pixel and depth slices.
pub struct BorrowedRenderTarget<'a> {
    pixels: &'a mut [u32],
    depths: &'a mut [f32],
    width: u32,
    height: u32,
}

impl<'a> BorrowedRenderTarget<'a> {
    /// Create a borrowed render target from caller-owned slices.
    ///
    /// # Errors
    /// Returns an error if the dimensions result in a capacity overflow or if the
    /// provided pixel and depth slices do not exactly match the requested dimensions.
    pub fn new(
        width: u32,
        height: u32,
        pixels: &'a mut [u32],
        depths: &'a mut [f32],
    ) -> Result<Self, &'static str> {
        let expected_len = width
            .checked_mul(height)
            .ok_or("buffer dimensions overflow")? as usize;

        if pixels.len() != expected_len {
            return Err("pixel slice length does not match dimensions");
        }

        if depths.len() != expected_len {
            return Err("depth slice length does not match dimensions");
        }

        Ok(Self {
            pixels,
            depths,
            width,
            height,
        })
    }

    /// Width of the borrowed render target in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the borrowed render target in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Read-only access to the color buffer.
    #[must_use]
    pub const fn pixels(&self) -> &[u32] {
        self.pixels
    }

    /// Mutable access to the color buffer.
    pub const fn pixels_mut(&mut self) -> &mut [u32] {
        self.pixels
    }

    /// Read-only access to the depth buffer.
    #[must_use]
    pub const fn depths(&self) -> &[f32] {
        self.depths
    }

    /// Mutable access to the depth buffer.
    pub const fn depths_mut(&mut self) -> &mut [f32] {
        self.depths
    }

    /// Split the target into mutable pixel and depth slices.
    pub const fn split_mut(&mut self) -> (&mut [u32], &mut [f32]) {
        (self.pixels, self.depths)
    }
}

#[cfg(test)]
mod tests {
    use super::super::RenderTarget;
    use super::*;

    #[test]
    fn borrowed_target_accepts_exact_length_slices() {
        let mut pixels = vec![0_u32; 4];
        let mut depths = vec![f32::INFINITY; 4];
        let target = BorrowedRenderTarget::new(2, 2, pixels.as_mut_slice(), depths.as_mut_slice())
            .expect("expected borrowed target to accept exact-length slices");

        assert_eq!(target.width(), 2);
        assert_eq!(target.height(), 2);
        assert_eq!(target.pixels().len(), 4);
        assert_eq!(target.depths().len(), 4);
    }

    #[test]
    fn borrowed_target_rejects_short_pixel_slice() {
        let mut pixels = vec![0_u32; 3];
        let mut depths = vec![f32::INFINITY; 4];

        assert!(
            BorrowedRenderTarget::new(2, 2, pixels.as_mut_slice(), depths.as_mut_slice()).is_err()
        );
    }

    #[test]
    fn borrowed_target_rejects_short_depth_slice() {
        let mut pixels = vec![0_u32; 4];
        let mut depths = vec![f32::INFINITY; 3];

        assert!(
            BorrowedRenderTarget::new(2, 2, pixels.as_mut_slice(), depths.as_mut_slice()).is_err()
        );
    }

    #[test]
    fn owned_render_target_can_borrow_mut() {
        let mut target = RenderTarget::new(2, 2).expect("valid target");
        let borrowed = target.borrow_mut();

        assert_eq!(borrowed.width(), 2);
        assert_eq!(borrowed.height(), 2);
        assert_eq!(borrowed.pixels().len(), 4);
        assert_eq!(borrowed.depths().len(), 4);
    }
}
