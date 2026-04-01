use std::cell::RefCell;

thread_local! {
    static BLACK_HOLE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}
