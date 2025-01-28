mod global_heap;

#[allow(unused)]
pub use global_heap::heap_statistics;

pub fn init() {
    global_heap::init();
}
