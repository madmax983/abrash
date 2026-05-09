//! Fuzzing harness for the OBJ Loader.
use abrash_core::obj_loader::load_obj;

#[doc(hidden)]
pub fn fuzz_load_obj(data: &[u8]) {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = load_obj(s);
    }
}
