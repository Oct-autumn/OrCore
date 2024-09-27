//! os/src/main.rs
//! The main source code
#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func
#![feature(alloc_error_handler)] //Use alloc_error_handler
#![feature(sync_unsafe_cell)] //Use sync_unsafe_cell

use core::arch::global_asm;

extern crate alloc;
extern crate bitflags;

use log::info;
use util::cpu;

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

const BOOT_HART: usize = 0;

/// 全局初始化，应由BOOT核完成
fn global_init(_device_tree_paddr: usize) {
    //初始化bss段
    init_bss();

    // 输出内核版本信息
    println!("");
    println!("< OrCore build: {} >", env!("CARGO_PKG_VERSION"),);

    // 初始化内存管理子模块
    println!("Initializing memory management...");
    mem::init();

    // 初始化日志子模块
    println!("Initializing log module...");
    kernel_log::init();

    // 初始化进程管理子模块
    task::add_initproc(); // 添加initproc任务

    loader::list_apps(); // 列出所有App

    info!("System init finished");
}

fn hart_init(hart_id: usize) {
    info!("Hello RISC-V! in hart {}", hart_id);

    unsafe {
        riscv::register::sstatus::set_sum();
    }

    // 初始化系统调用子模块
    info!("Init trap handler...");
    trap::init();

    // 启用内核空间
    mem::KERNEL_SPACE.clone().read().activate();

    // 初始化时钟中断
    info!("Init time interrupt...");
    trap::enable_timer_interrupt(); // 启用时钟中断
    util::time::set_next_timer(); // 设置下个时钟中断

    // kernel初始化完成，开始运行第一个任务
    info!("Hart init finished.");
    task::run_tasks(); // 运行任务

    unreachable!("Unreachable in other_hart_init!");
}

#[no_mangle] //disable mangle of func name 'rust_main'
pub fn rust_main(hart_id: usize, _device_tree_paddr: usize) -> ! {
    cpu::set_cpu_id(hart_id);
    if hart_id == BOOT_HART {
        global_init(_device_tree_paddr);
        // 启动其他核
        cpu::broadcast_ipi();
    }
    hart_init(hart_id);

    unreachable!("Unreachable in rust_main!");
}
