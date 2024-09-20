#![no_std]
#![feature(linkage)] // 启用弱链接特性

use sys_call::*;

#[macro_use]
pub mod console;
mod lang_items;
mod sys_call;

#[no_mangle]
#[link_section = ".text.entry"] // 定义该段为entry段，方便调整内存布局
                                // 值得注意的是，这段代码被linker放在了.text.entry段，整个SECTION的最开始处。
                                // 在batch执行时，jump过来的PC指针会直接开始执行这段代码。
pub extern "C" fn _start() -> ! {
    clear_bss(); //当使用半系统模拟时，注释掉
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
    let length = end_bss as usize - start_bss as usize;
    unsafe {
        core::slice::from_raw_parts_mut(start_bss as *mut u8, length).fill(0);
    }
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_next() -> isize {
    sys_yield()
}

pub fn get_time_msec() -> usize {
    let mut tv = TimeVal { sec: 0, usec: 0 };
    sys_get_time(&mut tv, 0);
    tv.usec / 1_000 + tv.sec * 1_000
}

pub fn get_time_usec() -> usize {
    let mut tv = TimeVal { sec: 0, usec: 0 };
    sys_get_time(&mut tv, 0);
    tv.usec + tv.sec * 1_000_000
}
