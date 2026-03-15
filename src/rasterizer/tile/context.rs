use crate::math::ScreenPoint;
use std::ops::{Deref, DerefMut};

/// Fixed-point vertex coordinates using 24.8 format (24 bits integer, 8 bits fractional).
///
/// This provides sub-pixel precision while using deterministic integer arithmetic.
/// The 24-bit integer part supports coordinates up to 16,777,215 pixels (well beyond 4K).
///
/// # Benefits over floating-point
///
/// - **Deterministic**: No floating-point rounding issues, always pixel-identical results
/// - **Faster**: Integer ALU operations are faster than float on most CPUs
/// - **Better SIMD**: AVX2 can process 16 i32 values vs 8 f32 values per instruction
///
/// # Format
///
/// - Fixed-point scale factor: 256 (2^8)
#[cfg(feature = "parallel")]
/// Wrapper for raw pointers to enable thread-safe parallel writes to non-overlapping regions.
///
/// SAFETY: This is safe because each thread writes to a non-overlapping region determined
/// by its tile coordinates (tx, ty). The tile renderer ensures that no two tiles overlap.
pub struct SendPtr<T>(pub *mut T);

#[cfg(feature = "parallel")]
impl<T> SendPtr<T> {
    /// SAFETY: Caller must ensure the index is within bounds and writes are to non-overlapping regions
    #[inline]
    pub unsafe fn write(&self, index: usize, value: T) {
        // SAFETY: Caller guarantees index is within bounds and writes are non-overlapping
        unsafe {
            *self.0.add(index) = value;
        }
    }
}

#[cfg(feature = "parallel")]
unsafe impl<T> Send for SendPtr<T> {}

#[cfg(feature = "parallel")]
unsafe impl<T> Sync for SendPtr<T> {}

#[allow(dead_code)]
pub struct AlignedBuffer<T> {
    _data: Vec<T>,
    pub ptr: *mut T,
    pub len: usize,
}

#[allow(dead_code)]
impl<T: Default + Copy> AlignedBuffer<T> {
    #[must_use]
    pub fn new(len: usize) -> Self {
        // We want 32-byte alignment.
        let align_bytes = 32;
        let elem_size = std::mem::size_of::<T>();
        // Ensure we allocate enough extra space to align the pointer
        // Worst case offset is align_bytes - 1. We need ceil((align_bytes)/elem_size) extra elements.
        let extra_elements = (align_bytes + elem_size - 1) / elem_size;

        let mut data = vec![T::default(); len + extra_elements];

        let start_ptr = data.as_mut_ptr();
        let start_addr = start_ptr as usize;

        // Calculate offset to next 32-byte boundary
        let offset_bytes = (align_bytes - (start_addr % align_bytes)) % align_bytes;
        // Assume elem_size divides align_bytes or at least offset_bytes (true for u32/f32 and align 32)
        let offset_elements = offset_bytes / elem_size;

        let ptr = unsafe { start_ptr.add(offset_elements) };

        Self {
            _data: data,
            ptr,
            len,
        }
    }
}

impl<T> Deref for AlignedBuffer<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> DerefMut for AlignedBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

unsafe impl<T: Send> Send for AlignedBuffer<T> {}

unsafe impl<T: Sync> Sync for AlignedBuffer<T> {}

/// Context for rendering a triangle into a tile.
pub struct TileContext<'a> {
    pub pixels: &'a mut [u32],
    pub depths: &'a mut [f32],
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub screen_x_max: i32,
}

/// Tile size in pixels. 32x32 = 1024 pixels * 4 bytes = 4KB per buffer.
pub const TILE_SIZE: u32 = 32;

impl TileContext<'_> {
    #[inline(always)]
    #[must_use]
    pub const fn get_indices(&self, x: i32, y: i32) -> usize {
        ((y - self.y0) as u32 * TILE_SIZE + (x - self.x0) as u32) as usize
    }

    #[inline(always)]
    #[must_use]
    pub const fn get_row_offset(&self, y: i32) -> usize {
        ((y - self.y0) as u32 * TILE_SIZE) as usize
    }
}

/// A compact screen point for storing vertices in `PreparedTriangle`.
///
/// Reduces memory usage by using i16 for coordinates (sufficient for up to 32k resolution)
/// and omitting unused `inv_w` for flat shading.
#[derive(Clone, Copy, Debug)]
pub struct CompactScreenPoint {
    pub x: i32,
    pub y: i32,
    pub z: f32,
}

impl CompactScreenPoint {
    #[must_use]
    pub fn to_screen_point(self, inv_w: f32) -> ScreenPoint {
        ScreenPoint {
            x: i32::from(self.x),
            y: i32::from(self.y),
            z: self.z,
            inv_w,
        }
    }
}
