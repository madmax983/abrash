
#![allow(warnings)]

#![allow(clippy::suspicious_operation_groupings)]
#![allow(clippy::imprecise_flops)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unused_parens)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(
    clippy::all,
    unused_variables,
    dead_code,
    unused_imports,
    unsafe_op_in_unsafe_fn,
    unused_mut
)]
//! Core math, geometry, and buffer types for the Abrash rendering engine.

pub mod clipping;
pub mod culling;
pub mod framebuffer;
pub mod geometry;
pub mod hiz_buffer;
pub mod math;
pub mod mesh;
pub mod obj_loader;
pub mod texture;
pub mod time;
pub mod utils;
pub mod zbuffer;

pub mod animatable;
pub mod bam;
pub mod color;
pub mod curve;
pub mod fixed16_16;
pub mod noise;
pub mod plane;
pub mod quat;
pub mod ray;
pub mod transform;

pub mod blitter;
pub mod easing;
pub mod gradient;
pub mod irect;
pub mod ivec;
pub mod obj_loader_havoc;
pub mod random;
pub mod sdf;
