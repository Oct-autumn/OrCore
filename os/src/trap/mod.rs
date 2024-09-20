mod context;
pub mod trap;

use log::{error, warn};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

pub use context::TrapContext;
use trap::__alltraps;

use crate::{
    syscall::syscall,
    task::{exit_current_and_run_next, suspend_current_and_run_next},
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
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // 获取中断原因
    let stval = stval::read(); // 获取stval寄存器的值(额外参数)
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // 时钟中断
            reset_next_timer();
            suspend_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            // 来自用户程序的系统调用
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            // 来自用户程序的内存访问异常
            warn!("PageFault in application, kernel killed it.");
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // 来自用户程序的非法指令
            warn!("IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        _ => {
            // 无法处理的中断
            error!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
            exit_current_and_run_next();
        }
    }
    cx
}

/// 启用时钟中断
pub fn enable_timer_interrupt() {
    unsafe {
        // 设置sie寄存器的STIE位，使能时钟中断
        // 避免S模式下时钟中断被屏蔽
        sie::set_stimer();
    }
}
