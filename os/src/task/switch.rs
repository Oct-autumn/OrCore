//! os/src/task/switch.rs
//!
//! 封装了任务切换的汇编代码__switch()，用于在任务之间切换上下文。
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

use super::TaskContext;

extern "C" {
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
