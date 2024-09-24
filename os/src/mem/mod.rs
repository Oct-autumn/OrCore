pub mod address;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod memory_set;
pub mod page_table;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use memory_set::MemorySet;

use crate::{println, sync::UPSafeCell};

lazy_static! {
    /// 内核内存空间
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub fn init() {
    // 初始化堆内存分配器
    println!("Initializing heap allocator...");
    heap_allocator::init_heap();
    heap_allocator::heap_test();
    // 初始化页帧分配器
    println!("Initializing frame allocator...");
    frame_allocator::init_frame_allocator();
    frame_allocator::frame_alloc_test();

    // 手动MMU测试
    page_table::mmu_test();

    // 启用内核内存空间
    KERNEL_SPACE.exclusive_access().activate();
    memory_set::remap_test();
    println!("Memory management initialized.");
}