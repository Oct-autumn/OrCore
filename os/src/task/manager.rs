use crate::sync::UPSafeCell;

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use lazy_static::lazy_static;

use super::process::ProcessControlBlock;

lazy_static! {
    static ref PROCESS_MANAGER: UPSafeCell<ProcessManager> =
        unsafe { UPSafeCell::new(ProcessManager::new()) };
}

/// 任务管理器
///
/// 任务管理器负责管理所有任务，包括任务的创建、删除、切换等。
pub struct ProcessManager {
    /// 就绪队列<br>
    /// 这里采用轮转调度，采用就绪队列存储就绪任务。
    /// TODO: （多核）支持多级轮转调度
    ready: VecDeque<Arc<ProcessControlBlock>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
        }
    }

    /// 添加一个任务到就绪队列
    pub fn add(&mut self, task: Arc<ProcessControlBlock>) {
        self.ready.push_back(task);
    }

    /// 从就绪队列中取出一个任务
    pub fn fetch(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready.pop_front()
    }
}

pub fn add_ready_process(task: Arc<ProcessControlBlock>) {
    PROCESS_MANAGER.exclusive_access().add(task);
}

pub fn fetch_process() -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.exclusive_access().fetch()
}
