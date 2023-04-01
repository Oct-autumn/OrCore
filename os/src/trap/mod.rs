use core::arch::global_asm;

use log::{error, warn};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

pub use context::TrapContext;

use crate::batch::run_next_app;
use crate::syscall::syscall;

mod context;

global_asm!(include_str!("trap.S"));

/// 初始化中断处理
pub fn init() {
    extern "C" { fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// 中断处理函数
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();    // 获取中断原因
    let stval = stval::read();          // 获取stval寄存器的值(额外参数)
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 来自用户程序的系统调用
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            // 来自用户程序的内存访问异常
            warn!("PageFault in application, kernel killed it.");
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // 来自用户程序的非法指令
            warn!("IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }
        _ => {
            // 无法处理的中断
            error!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
            run_next_app();
        }
    }
    cx
}

