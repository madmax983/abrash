//! Post-processing effects.
//!
//! Functions to apply full-screen effects to a `Framebuffer`.
//!
//! # Examples
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::post_process::{apply_grayscale, apply_scanlines, apply_invert};
//!
//! let mut fb = Framebuffer::new(100, 100).unwrap();
//! // ... render something ...
//!
//! // Apply effects
//! apply_grayscale(&mut fb);
//! apply_scanlines(&mut fb);
//! apply_invert(&mut fb);
//! ```

pub mod bloom;
pub mod blur;
pub mod filters;
pub mod sobel;
pub mod ssao;

pub use self::bloom::*;
pub use self::blur::*;
pub use self::filters::*;
pub use self::sobel::*;
pub use self::ssao::*;
