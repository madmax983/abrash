#![allow(clippy::manual_is_power_of_two)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::manual_ilog2)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::use_self)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::missing_const_for_fn)]
#![allow(
    clippy::imprecise_flops,
    clippy::items_after_statements,
    clippy::must_use_candidate,
    clippy::suspicious_operation_groupings,
    clippy::unreadable_literal,
    clippy::branches_sharing_code
)]
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
