use alloc::collections::vec_deque::VecDeque;
use lazy_static::lazy_static;

use crate::{config, sync::UPSafeCell};

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) };
}

pub struct PidHandle(pub usize);

impl From<usize> for PidHandle {
    fn from(pid: usize) -> Self {
        Self(pid)
    }
}

impl From<PidHandle> for usize {
    fn from(pid_handle: PidHandle) -> usize {
        pid_handle.0
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

struct PidAllocator {
    current: usize,
    recycled: VecDeque<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: VecDeque::new(),
        }
    }

    pub fn alloc(&mut self) -> PidHandle {
        if self.recycled.len() > config::MIN_PID_RECYCLE {
            let pid = self.recycled.pop_front().unwrap();
            PidHandle(pid)
        } else {
            let pid = self.current;
            self.current += 1;
            PidHandle(pid)
        }
    }

    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            self.recycled.iter().all(|&p| p != pid),
            "pid {} has been recycled",
            pid
        );
        self.recycled.push_back(pid.into());
    }
}

/// 分配一个PID
pub fn alloc_pid() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}

/// 释放一个PID
#[allow(unused)]
pub fn dealloc_pid(pid: usize) {
    PID_ALLOCATOR.exclusive_access().dealloc(pid);
}
