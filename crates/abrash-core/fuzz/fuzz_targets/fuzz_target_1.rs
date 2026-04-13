#![no_main]

use libfuzzer_sys::fuzz_target;
use abrash_core::obj_loader::load_obj;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = load_obj(s);
    }
});
