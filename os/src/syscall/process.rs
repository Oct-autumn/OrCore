//! os/src/syscall/process.rs <br>
//! process related syscall

use log::*;

use crate::batch::run_next_app;

pub fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    run_next_app()
}