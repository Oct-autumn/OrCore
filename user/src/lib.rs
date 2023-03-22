#![no_std]
#![feature(linkage)]            // 启用弱链接特性
#![feature(panic_info_message)] // 启用panic_info_message特性

use sys_call::*;

#[macro_use]
pub mod console;
mod lang_items;
mod sys_call;

#[no_mangle]
#[link_section = ".text.entry"] // 定义该段为entry段，方便调整内存布局
pub extern "C" fn _start() -> ! {
    //clear_bss();  //当使用半系统模拟时，注释掉
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"] // 弱链接，使得当用户程序没有main函数时自动链接至此main函数
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

// 清理bss段
fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}