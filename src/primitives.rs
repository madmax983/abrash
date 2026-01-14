//! Rasterization primitives.
//!
//! Software rendering functions that operate on framebuffers.
//! All primitives perform bounds checking.

use crate::framebuffer::Framebuffer;

pub fn plot_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32) {
    fb.set_pixel(x, y, color);
}
