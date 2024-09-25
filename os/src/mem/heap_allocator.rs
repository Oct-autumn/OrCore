//! os/src/mem/heap_allocator.rs
//!
//! 堆内存分配器

use crate::{config::KERNEL_HEAP_SIZE, println, sbi_call::shutdown};
use buddy_system_allocator::LockedHeap;

// 定义堆内存分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init_heap() {
    unsafe {
        #[allow(static_mut_refs)] // 禁用static_mut_refs警告
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    println!("Heap allocation error, layout = {:?}", layout);
    shutdown()
}

/// 堆内存分配器测试
/// 测试堆内存分配和回收
#[allow(unused)]
pub fn heap_test() {
    println!("running heap_test...");
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
