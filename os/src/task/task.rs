//! os/src/task/task.rs

use log::debug;

use crate::{
    config::{self, kernel_stack_position},
    mem::{
        address::{PhysPageNum, VirtAddr},
        memory_set::{MemorySet, SegPermission},
        KERNEL_SPACE,
    },
    trap::{trap_handler, TrapContext},
};

use super::context::TaskContext;

/// 任务状态
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    #[allow(unused)]
    UnInit, // 未初始化
    Ready,   // 就绪态
    Running, // 运行态
    Exited,  // 终止态
}

/// 任务控制块（TCB）
///
/// 包含任务运行状态和任务上下文
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    #[allow(unused)]
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 申请内存空间
        let (memory_set, user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data);
        debug!(
            "new task: ptb_token = {:#x}, user_sp = {:#x}, entry_point = {:#x}",
            memory_set.token(),
            user_sp,
            entry_point
        );
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(config::TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // 申请内核栈空间
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            SegPermission::R | SegPermission::W,
        );
        debug!(
            "kernel stack: {:#x} - {:#x}",
            kernel_stack_bottom, kernel_stack_top
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    /// 获取中断上下文
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// 获取应用程序的页表token
    #[allow(unused)]
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
}
