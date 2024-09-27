//! os/src/batch.rs <br>
//! The batch system
mod context;
mod kernel_stack;
mod manager;
mod pid;
mod process;
mod processor;
mod switch;

use alloc::sync::Arc;
use context::ProcessContext;
use lazy_static::lazy_static;
use log::*;
use process::ProcessControlBlock;

use crate::loader::get_elf_data_by_name;

pub use processor::{current_process, current_process_token, current_trap_cx, run_tasks};

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = Arc::new(ProcessControlBlock::new(
        get_elf_data_by_name("initproc").unwrap()  // 获取initproc的元数据
    ));
}

/// 将初始化进程添加到进程管理器中
pub fn add_initproc() {
    info!("Adding initproc to the process manager...");
    manager::add_ready_process(INITPROC.clone());
}

/// 暂停当前任务并运行下一个任务
pub fn suspend_current_and_run_next() {
    // 取出当前任务
    let cp = processor::take_current_process().unwrap();

    let mut cpi = cp.inner_write();
    // 获取当前任务的上下文指针
    let process_cx_ptr = &mut cpi.process_cx as *mut ProcessContext;
    // 将当前任务标记为“就绪”
    cpi.process_status = process::ProcessStatus::Ready;

    // 以下释放引用是必须的
    // schedule会切换到新的任务，如果不在这里释放引用，会导致引用一直存在，造成内存泄漏
    drop(cpi); // 释放mut引用

    manager::add_ready_process(cp); // 将进程添加到就绪队列中

    processor::schedule(process_cx_ptr);
}

/// 退出当前任务并运行下一个任务
pub fn exit_current_and_run_next(exit_code: i32) {
    let cp = processor::take_current_process().unwrap();
    let mut cpi = cp.inner_write();
    cpi.process_status = process::ProcessStatus::Zombie; // 将当前任务标记为“僵尸态”
    cpi.exit_code = exit_code; // 设置返回值

    {
        // 为了使当前进程的子进程在父进程结束后仍然运行，将Initproc设置为其父进程
        let mut initproc_inner = INITPROC.inner_write();
        for child in cpi.children.iter() {
            child.inner_write().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }

    cpi.children.clear(); // 清空当前任务的子进程列表
    cpi.memory_set.recycle(); // 回收当前任务的内存空间

    // 以下两步释放引用是必须的
    // schedule会切换到新的任务，如果不在这里释放引用，会导致引用一直存在，造成内存泄漏
    drop(cpi); // 释放inner引用
    drop(cp); // 释放cp引用

    let mut _unused = ProcessContext::zero_init(); // 用于接收当前任务的上下文（实际上当前任务不会返回）
    processor::schedule(&mut _unused as *mut _);
}

/// 添加一个任务到就绪队列
pub fn add_task(task: Arc<ProcessControlBlock>) {
    assert!(task.inner_read().process_status == process::ProcessStatus::Ready);
    manager::add_ready_process(task);
}
