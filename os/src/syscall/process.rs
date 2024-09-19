//! os/src/syscall/process.rs <br>
//! process related syscall

use log::*;

use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};

pub fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    info!("Yield current application");
    suspend_current_and_run_next();
    0
}
