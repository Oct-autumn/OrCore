//! os/src/trap/mod.rs
//!
//! 中断处理模块
//! 包含中断分发

mod context;
pub mod trap;

use core::arch::asm;

use log::{error, trace, warn};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

pub use context::TrapContext;
use trap::{__alltraps, __restore};

use crate::{
    config::{self},
    syscall::syscall,
    task,
    util::time::reset_next_timer,
};

/// 初始化中断处理
pub fn init() {
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// 中断处理函数
#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let cx = task::current_trap_cx();
    let scause = scause::read(); // 获取中断原因
    let stval = stval::read(); // 获取stval寄存器的值(额外参数)
    trace!(
        "A User trap was caught! scause: {:?}, stval: {:#x}",
        scause.cause(),
        stval
    );
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // 时钟中断
            reset_next_timer();
            task::suspend_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            // 来自用户程序的系统调用
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            // 来自用户程序的内存访问异常
            warn!("PageFault in application, kernel killed it.");
            task::exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // 来自用户程序的非法指令
            warn!("IllegalInstruction in application, kernel killed it.");
            task::exit_current_and_run_next();
        }
        _ => {
            // 无法处理的中断
            error!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
            task::exit_current_and_run_next();
        }
    }
    trap_return()
}

/// 启用时钟中断
pub fn enable_timer_interrupt() {
    unsafe {
        // 设置sie寄存器的STIE位，使能时钟中断
        // 避免S模式下时钟中断被屏蔽
        sie::set_stimer();
    }
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    let scause = scause::read();
    let stval = stval::read();
    panic!(
        "Unhandled Kernel trap: {:?}, stval: {:#x}",
        scause.cause(),
        stval
    );
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(config::TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = config::TRAP_CONTEXT;
    let user_satp = task::current_app_token();
    let restore_va = __restore as usize - __alltraps as usize + config::TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}
