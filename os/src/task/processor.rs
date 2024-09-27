use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;

use crate::{config, sync::RwLock, util::cpu};

use super::{
    context::ProcessContext,
    manager::fetch_process,
    process::{ProcessControlBlock, ProcessStatus},
    switch::__switch,
};

lazy_static! {
    /// 处理器实例
    pub static ref PROCESSORS: Vec<RwLock<Processor>> = {
        let mut processors = Vec::new();

        // 创建处理器实例
        for _ in 0..config::CPU_NUM {
            processors.push(RwLock::new(Processor::new()));
        }

        processors
    };
}

/// idle任务：当没有任务可以运行时，运行idle任务<br>
/// 该任务将尝试从进程管理器中获取一个ready的进程并运行它。
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSORS[cpu::hart_id()].write();

        // 尝试从进程管理器中获取一个进程
        if let Some(process) = fetch_process() {
            // 成功获取，切换到该进程

            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            let mut process_inner = process.inner_write();
            let next_task_cx_ptr = &process_inner.process_cx as *const ProcessContext;
            process_inner.process_status = ProcessStatus::Running; // 设置进程状态为Running
            drop(process_inner);
            processor.current = Some(process); // 设置当前进程
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}

/// 调度函数：当发生进程调度时，运行调度函数<br>
/// 将当前任务切换到idle任务
pub fn schedule(switched_task_cx_ptr: *mut ProcessContext) {
    let mut processor = PROCESSORS[cpu::hart_id()].write();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

/// 处理器实例
pub struct Processor {
    /// 当前正在运行的进程
    current: Option<Arc<ProcessControlBlock>>,
    /// idle进程的上下文
    idle_task_cx: ProcessContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: ProcessContext::zero_init(),
        }
    }

    /// 取出当前进程
    pub fn take_current(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.current.take()
    }

    /// 获取当前进程PCB指针的一份拷贝
    pub fn current(&self) -> Option<Arc<ProcessControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }

    fn get_idle_task_cx_ptr(&mut self) -> *mut ProcessContext {
        &mut self.idle_task_cx as *mut _
    }
}

/// 取出cpu当前的进程
pub fn take_current_process() -> Option<Arc<ProcessControlBlock>> {
    PROCESSORS[cpu::hart_id()].write().take_current()
}

/// 获取当前进程PCB指针的一份拷贝
pub fn current_process() -> Option<Arc<ProcessControlBlock>> {
    PROCESSORS[cpu::hart_id()].read().current()
}

/// 获取当前进程的页表token
pub fn current_process_token() -> usize {
    current_process().unwrap().inner_read().get_user_token()
}

/// 获取当前进程的中断上下文
pub fn current_trap_cx() -> &'static mut crate::trap::TrapContext {
    current_process().unwrap().inner_read().get_trap_cx()
}
