//! os/src/main.rs
//! The main source code
#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func
#![feature(alloc_error_handler)] //Use alloc_error_handler
#![feature(sync_unsafe_cell)] //Use sync_unsafe_cell

use core::{arch::global_asm, panic};

extern crate alloc;
extern crate bitflags;

use log::info;

use crate::console::print;

mod config;
mod console;
mod error;
mod kernel_log;
mod lang_items;
mod loader;
mod mem;
mod sbi_call;
mod sync;
mod syscall;
mod task;
mod trap;
mod util;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S")); //将App链入内核

/// 初始化（清零）bss段
fn init_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        // 填零处理
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle] //disable mangle of func name 'rust_main'
pub fn rust_main() -> ! {
    init_bss(); //初始化bss段
    println!("[Test] Hello, world!"); // English test
    kernel_log::init();

    // 初始化内存管理子模块
    println!("Initializing memory management...");
    mem::init();
    // 初始化日志子模块
    println!("Initializing log module...");
    // 初始化系统调用子模块
    info!("Init trap handler...");
    trap::init();
    // 初始化时钟中断
    info!("Init time interrupt...");
    trap::enable_timer_interrupt(); // 启用时钟中断
    util::time::reset_next_timer(); // 设置下个时钟中断

    // kernel初始化完成，开始运行第一个任务
    info!("System init finished, start first task...");
    task::run_first_task();

    panic!("Unreachable in rust_main!");
}
