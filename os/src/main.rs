//! os/src/main.rs
//! The main source code
#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

use core::{arch::global_asm, panic};

use log::*;

use crate::console::print;

mod config;
mod console;
mod kernel_log;
mod lang_items;
mod loader;
mod sbi_call;
mod sync;
mod syscall;
mod task;
mod trap;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S")); //将App链入内核

#[no_mangle] //disable mangle of func name 'rust_main'
pub fn rust_main() -> ! {
    init_bss(); //初始化bss段
    kernel_log::init();
    println!("[Test] Hello, world!"); // English test
    println!("[Test] 你好，世界！"); // 中文测试
    error!("[Test] ERROR log level"); // 内核遇到了可恢复的错误，但无法确定是否会影响系统稳定性
    warn!("[Test] WARN log level"); // 内核遇到了可恢复的错误，但不会影响系统稳定性
    info!("[Test] INFO log level"); // 重要的信息，但不是错误信息
    debug!("[Test] DEBUG log level"); // 用于调试的信息
    trace!("[Test] TRACE log level"); // 用于调试的详细信息，会追踪到每个步骤

    // 调用AppManager
    info!("Init trap handler.");
    trap::init();
    info!("Init task system.");
    info!("loading apps...");
    loader::load_apps();
    task::run_first_task();

    panic!("Unreachable in rust_main!");
}

fn init_bss() {
    // init the .bss section
    // use the agreement in C lang to find the section address
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    // iterator to init the section
    (start_bss as usize..end_bss as usize)
        .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
}
