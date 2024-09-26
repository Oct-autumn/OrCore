//! os/src/task/task.rs

use alloc::{sync::Arc, sync::Weak, vec::Vec};
use log::*;

use crate::{
    config,
    error::Result,
    mem::{
        address::{PhysPageNum, VirtAddr},
        memory_set::MemorySet,
        KERNEL_SPACE,
    },
    sync::{SpinLock, SpinLockGuard},
    task::pid,
    trap::{trap_handler, TrapContext},
};

use super::{context::ProcessContext, kernel_stack::KernelStack, pid::PidHandle};

/// 任务状态
#[derive(Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    /// 未初始化
    /// 任务还没有初始化完成
    #[allow(unused)]
    UnInit,
    /// 就绪态<br>
    /// 任务已经准备好，等待调度
    Ready,
    /// 运行态<br>
    /// 任务正在运行
    Running,
    /// 阻塞态
    /// 任务因为某些原因无法继续运行
    #[allow(unused)]
    Blocked,
    /// 终止态
    /// 任务已经结束
    #[allow(unused)]
    Exited,
    /// 僵尸态
    /// 任务已经结束，但是父进程还没有回收
    Zombie,
}

/// 进程控制块内部数据（进程安全）
pub struct ProcessControlBlockInner {
    /// 中断上下文所在的物理页号
    pub trap_cx_ppn: PhysPageNum,
    /// 用户栈
    pub base_size: usize,
    /// 进程上下文
    pub process_cx: ProcessContext,
    /// 进程状态
    pub process_status: ProcessStatus,
    /// 进程内存工作集
    pub memory_set: MemorySet,
    /// 进程的父进程PCB
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// 子进程PCB
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// 退出代码
    pub exit_code: i32,
}

/// 进程控制块
pub struct ProcessControlBlock {
    /// 进程ID
    pub pid: PidHandle,
    /// 内核栈
    pub kernel_stack: KernelStack,
    /// 进程控制块内部数据（进程安全）
    inner: SpinLock<ProcessControlBlockInner>,
}

impl ProcessControlBlockInner {
    /// 获取中断上下文
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// 获取应用程序的页表token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    /// 获取进程状态
    #[allow(unused)]
    pub fn get_status(&self) -> ProcessStatus {
        self.process_status
    }

    /// 快捷判断：进程是否为僵尸态
    pub fn is_zombie(&self) -> bool {
        self.process_status == ProcessStatus::Zombie
    }
}

impl ProcessControlBlock {
    /// 从elf_data创建新进程<br>
    /// 本函数仅用于生成initproc
    pub fn new(elf_data: &[u8]) -> Self {
        // 申请内存空间
        let (memory_set, user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data)
            .map_err(|e| {
                panic!("Failed to create initproc: {:?}", e); // 若内存分配失败，则直接panic
            })
            .unwrap();
        debug!(
            "new process: ptb_token = {:#x}, user_sp = {:#x}, entry_point = {:#x}",
            memory_set.token(),
            user_sp,
            entry_point
        );
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(config::TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 申请PID
        let pid = pid::alloc_pid();
        // 申请内核栈空间
        let kernel_stack = KernelStack::new(&pid)
            .map_err(|e| {
                panic!("Failed to alloc kernel stack: {:?}", e); // 若内存分配失败，则直接panic
            })
            .unwrap();
        let kernel_stack_top = kernel_stack.get_top(); // 内核栈顶

        // 创建PCB
        let pcb = Self {
            pid,
            kernel_stack,
            inner: unsafe {
                SpinLock::new(ProcessControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    process_cx: ProcessContext::goto_trap_return(kernel_stack_top),
                    process_status: ProcessStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        };

        // 初始化中断上下文
        let trap_cx = pcb.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        pcb
    }

    /// 从elf_data执行新程序
    pub fn exec(&self, elf_data: &[u8]) -> Result<()> {
        let (memory_set, user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data)?;
        debug!(
            "exec process: ptb_token = {:#x}, user_sp = {:#x}, entry_point = {:#x}",
            memory_set.token(),
            user_sp,
            entry_point
        );

        // 中断上下文物理页
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(config::TRAP_CONTEXT).into())?
            .ppn();

        let mut inner = self.inner_exclusive_access();
        // 更换内存空间
        inner.memory_set = memory_set;
        // 更换中断上下文
        inner.trap_cx_ppn = trap_cx_ppn;

        // 初始化中断上下文
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        Ok(())
    }

    /// 复刻当前进程，生成子进程
    pub fn fork(self: &Arc<ProcessControlBlock>) -> Result<Arc<ProcessControlBlock>> {
        let mut inner = self.inner_exclusive_access();

        // 复制内存空间
        let memory_set = MemorySet::from_existed(&inner.memory_set)?;
        // 中断上下文物理页
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(config::TRAP_CONTEXT).into())?
            .ppn();

        // 申请PID
        let pid = pid::alloc_pid();
        // 申请内核栈空间
        let kernel_stack = KernelStack::new(&pid)?;
        let kernel_stack_top = kernel_stack.get_top(); // 内核栈顶

        // 创建PCB
        let pcb = Arc::new(Self {
            pid,
            kernel_stack,
            inner: unsafe {
                SpinLock::new(ProcessControlBlockInner {
                    trap_cx_ppn,
                    base_size: inner.base_size,
                    process_cx: ProcessContext::goto_trap_return(kernel_stack_top),
                    process_status: ProcessStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)), // 父进程
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        });

        // 将子进程加入父进程的children列表
        inner.children.push(pcb.clone());

        // 修改中断上下文中的内核栈指针
        let trap_cx = pcb.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        Ok(pcb)
    }

    /// 获取内部数据的共享引用
    pub fn inner_exclusive_access(&self) -> SpinLockGuard<'_, ProcessControlBlockInner> {
        self.inner.lock()
    }

    /// 获取进程ID
    pub fn get_pid(&self) -> usize {
        self.pid.0
    }
}
