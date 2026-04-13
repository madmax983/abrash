/// A render target backed by caller-owned pixel and depth slices.
pub struct BorrowedRenderTarget<'a> {
    pixels: &'a mut [u32],
    depths: &'a mut [f32],
    width: u32,
    height: u32,
}

impl<'a> BorrowedRenderTarget<'a> {
    /// Create a borrowed render target from caller-owned slices.
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
    pub fn pixels(&self) -> &[u32] {
        self.pixels
    }

    /// Mutable access to the color buffer.
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        self.pixels
    }

    /// Read-only access to the depth buffer.
    #[must_use]
    pub fn depths(&self) -> &[f32] {
        self.depths
    }

    /// Mutable access to the depth buffer.
    pub fn depths_mut(&mut self) -> &mut [f32] {
        self.depths
    }

    /// Split the target into mutable pixel and depth slices.
    pub fn split_mut(&mut self) -> (&mut [u32], &mut [f32]) {
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
