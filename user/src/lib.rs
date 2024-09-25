#![no_std]
#![feature(linkage)] // 启用弱链接特性
#![feature(alloc_error_handler)] // 启用alloc_error_handler特性

use core::u8;

use buddy_system_allocator::LockedHeap;
pub use syscall::*;

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

const USER_HEAP_SIZE: usize = 0x4000;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"] // 定义该段为entry段，方便调整内存布局
                                // 值得注意的是，这段代码被linker放在了.text.entry段，整个SECTION的最开始处。
                                // 在batch执行时，jump过来的PC指针会直接开始执行这段代码。
pub extern "C" fn _start() -> ! {
    unsafe {
        #[allow(static_mut_refs)] // 禁用static_mut_refs警告
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    exit(main());
    unreachable!("unreachable after sys_exit!");
}

#[linkage = "weak"] // 弱链接，使得当用户程序没有main函数时自动链接至此main函数
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}
