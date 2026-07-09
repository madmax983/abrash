#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Extreme edge-case bounds and load testing for the OBJ Loader.
#[cfg(test)]
mod tests {
    use crate::obj_loader::load_obj;

    #[test]
    fn havoc_invalid_vertex_index() {
        let obj = "v 1.0 1.0 1.0\nv 1.0 1.0 1.0\nf 1 2 4";
        assert!(load_obj(obj).is_err());
    }

    #[test]
    fn havoc_huge_vertex_index() {
        let obj = "v 1.0 1.0 1.0\nv 1.0 1.0 1.0\nf 1 2 999999999";
        assert!(load_obj(obj).is_err());
    }

    #[test]
    fn havoc_huge_vertex_count() {
        let mut obj = String::new();
        for _ in 0..100 {
            obj.push_str("v 1.0 1.0 1.0\n");
        }
        for _ in 0..100 {
            obj.push_str("f 1 2 3\n");
        }
        let _ = load_obj(&obj);
    }

    #[test]
    fn havoc_sentinel_collision() {
        let obj = "v 1.0 1.0 1.0\n\nf 1048576 1 1\n";
        let _ = load_obj(obj);
    }
}
