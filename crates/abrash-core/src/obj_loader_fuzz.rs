#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Fuzzing harness for the OBJ Loader.
use abrash_core::obj_loader::load_obj;

#[doc(hidden)]
pub fn fuzz_load_obj(data: &[u8]) {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = load_obj(s);
    }
}
