//! os/src/batch.rs <br>
//! The batch system
mod switch;
mod task;

use lazy_static::lazy_static;
use log::*;
use switch::__switch;

use crate::config::MAX_APP_NUM;
use crate::loader::get_num_app;
use crate::loader::stack::init_app_cx;
use crate::sync::UPSafeCell;

use task::{TaskContext, TaskControlBlock, TaskStatus};

/// 任务管理器
///
/// 任务管理器负责管理所有任务，包括任务的创建、删除、切换等。
pub struct TaskManager {
    num_app: usize,
    // 因为可能会有多个任务同时访问任务管理器，所以这里使用了`UPSafeCell`进行封装。
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器内部数据
///
/// 任务管理器内部数据包括所有任务的控制块和当前任务的索引。
struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

// 全局的任务管理器
lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        trace!("Initializing TASK_MANAGER...");

        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock::default(); MAX_APP_NUM];

        for i in 0..num_app {
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
        }

        unsafe {
            TaskManager {
                num_app,
                inner: UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                }),
            }
        }
    };
}

impl TaskManager {
    /// 运行顺位第一个任务
    fn run_first_task(&self) -> ! {
        trace!("Going to run the first app...");
        // 此时TASK_MANAGER应当已经初始化完成
        let mut task_manager_inner = self.inner.exclusive_access();
        let task0 = &mut task_manager_inner.tasks[0];
        task0.task_status = TaskStatus::Running; // 将任务状态设置为“运行态”
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(task_manager_inner); // 释放mut引用

        let mut _deprecate = TaskContext::zero_init(); // 用于接收当前任务的上下文（实际上当前任务不会返回）
        debug!("Jumping to task_0...");
        unsafe {
            __switch(&mut _deprecate as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("Unreachable in task::run_first_task!");
    }

    /// 将当前任务标记为“就绪”
    fn mark_current_task_as_ready(&self) {
        let mut task_manager_inner = self.inner.exclusive_access();
        let current_task = task_manager_inner.current_task;
        task_manager_inner.tasks[current_task].task_status = TaskStatus::Ready;
    }

    /// 将当前任务标记为“完成”
    fn mark_current_task_as_exited(&self) {
        let mut task_manager_inner = self.inner.exclusive_access();
        let current_task = task_manager_inner.current_task;
        task_manager_inner.tasks[current_task].task_status = TaskStatus::Exited;
    }

    /// 查找下一个可运行的任务
    ///
    /// 从当前任务开始，查找下一个状态为“就绪”的任务。
    /// 如果所有任务都不可运行，则返回`None`
    fn find_next_task(&self) -> Option<usize> {
        let task_manager_inner = self.inner.exclusive_access();
        let num_app = self.num_app;
        let current_task = task_manager_inner.current_task;
        let mut next_task = (current_task + 1) % num_app;
        // 从当前任务开始，查找下一个状态为“就绪”的任务
        while next_task != current_task {
            if task_manager_inner.tasks[next_task].task_status == TaskStatus::Ready {
                return Some(next_task);
            }
            next_task = (next_task + 1) % num_app;
        }
        // 如果没有其他任务可运行，判断当前任务是否为“就绪”状态
        if task_manager_inner.tasks[current_task].task_status == TaskStatus::Ready {
            Some(current_task)
        } else {
            None
        }
    }

    /// 运行下一个任务
    ///
    /// 如果没有下一个任务可运行，则会Panic
    fn run_next_task(&self) {
        if let Some(next_task_id) = self.find_next_task() {
            if next_task_id == self.inner.exclusive_access().current_task {
                // 如果下一个任务就是当前任务，则直接返回
                return;
            }

            trace!("Going to run next task...");

            let mut task_manager_inner = self.inner.exclusive_access();
            let current_task_id = task_manager_inner.current_task;
            // 将下一个任务的状态设置为“运行态”
            task_manager_inner.tasks[next_task_id].task_status = TaskStatus::Running;
            // 切换当前运行的任务id
            task_manager_inner.current_task = next_task_id;

            // 获取当前任务的上下文指针和下一个任务的上下文指针
            let current_task_cx_ptr =
                &mut task_manager_inner.tasks[current_task_id].task_cx as *mut TaskContext;
            let next_task_cx_ptr =
                &task_manager_inner.tasks[next_task_id].task_cx as *const TaskContext;

            drop(task_manager_inner); // 释放mut引用

            debug!("Jumping to task_{}...", next_task_id);

            //调用__switch func切换任务
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
                // 该操作将切换任务上下文，跳转到用户态，执行用户程序，不会返回
            }
        } else {
            panic!("No more tasks to run!");
        }
    }
}

/// 运行第一个任务
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// 切换运行下一任务
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// 将当前任务标记为“就绪”
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_task_as_ready();
}

/// 将当前任务标记为“完成”
fn mark_current_exited() {
    TASK_MANAGER.mark_current_task_as_exited();
}

/// 任务切换(将当前任务挂起，运行下一任务)
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// 任务切换(结束当前任务，运行下一任务)
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
