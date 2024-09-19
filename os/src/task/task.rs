//! os/src/task/task.rs

use crate::trap::trap::__restore;

/// 任务状态
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,  // 未初始化
    Ready,   // 就绪态
    Running, // 运行态
    Exited,  // 终止态
}

/// 任务上下文
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        TaskContext {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 为任务设置上下文
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        TaskContext {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}

/// 任务控制块（TCB）
///
/// 包含任务运行状态和任务上下文
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
}

impl TaskControlBlock {
    pub fn default() -> Self {
        TaskControlBlock {
            task_status: TaskStatus::UnInit,
            task_cx: TaskContext::zero_init(),
        }
    }
}
