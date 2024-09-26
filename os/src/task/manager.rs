use crate::sync::SpinLock;

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use lazy_static::lazy_static;

use super::process::ProcessControlBlock;

lazy_static! {
    /// 进程管理器实例
    ///
    /// 鉴于进程管理器基本没有读写竞争，因此使用自旋锁减小开销
    static ref PROCESS_MANAGER: SpinLock<ProcessManager> = SpinLock::new(ProcessManager::new());
}

/// 进程管理器
///
/// 进程管理器负责管理就绪状态和阻塞状态的进程。
pub struct ProcessManager {
    /// 就绪队列<br>
    /// 这里采用轮转调度，采用就绪队列存储就绪任务。
    /// TODO: （多核）支持多级轮转调度
    ready: VecDeque<Arc<ProcessControlBlock>>,
    /// 阻塞队列<br>
    /// 阻塞队列存储阻塞任务。
    /// TODO: 支持阻塞
    #[allow(unused)]
    blocked: VecDeque<Arc<ProcessControlBlock>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            blocked: VecDeque::new(),
        }
    }

    /// 添加一个任务到就绪队列
    pub fn add(&mut self, task: Arc<ProcessControlBlock>) {
        assert!(task.inner_read().is_ready());
        self.ready.push_back(task);
    }

    /// 从就绪队列中取出一个任务
    pub fn fetch(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready.pop_front()
    }
}

pub fn add_ready_process(task: Arc<ProcessControlBlock>) {
    PROCESS_MANAGER.lock().add(task);
}

pub fn fetch_process() -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.lock().fetch()
}
