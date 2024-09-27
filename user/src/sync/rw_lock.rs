//! os/src/sync/rw_lock.rs
//! 读写锁，内部使用自旋锁实现

use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

// 实现一个读写锁RwLock，内部包含一个UnsafeCell，用于存储被保护的数据
// 要求：
//      不保证读写锁的公平性
//      允许同时多读，但不允许同时读写、同时写写

/// 读写锁
pub struct RwLock<T> {
    readers: AtomicUsize,
    writer: AtomicUsize,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for RwLock<T> where T: Send {}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        RwLock {
            readers: AtomicUsize::new(0),
            writer: AtomicUsize::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// 获取读锁
    pub fn read(&self) -> RwLockReadGuard<T> {
        loop {
            // 等待写锁释放
            while self.writer.load(Ordering::Acquire) != 0 {}

            // 增加读者计数
            self.readers.fetch_add(1, Ordering::Acquire);

            // 再次检查写锁，确保没有写者在等待
            if self.writer.load(Ordering::Acquire) == 0 {
                break;
            }

            // 如果有写者在等待，减少读者计数并重试
            self.readers.fetch_sub(1, Ordering::Release);
        }

        RwLockReadGuard { lock: self }
    }

    /// 获取写锁
    pub fn write(&self) -> RwLockWriteGuard<T> {
        // 等待其他写者释放锁
        while self
            .writer
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {}

        // 等待所有读者释放锁
        while self.readers.load(Ordering::Acquire) != 0 {}

        RwLockWriteGuard { lock: self }
    }

    /// 从读锁升级为写锁
    pub fn upgrade<'a>(read_guard: RwLockReadGuard<'a, T>) -> RwLockWriteGuard<'a, T> {
        // 等待其他写者释放锁
        while read_guard
            .lock
            .writer
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {}

        // 等待所有其它读者释放锁
        // 最终只剩下read_guard一个读者，而在函数返回后，read_guard会被销毁，不再存在读者
        while read_guard.lock.readers.load(Ordering::Acquire) != 1 {}

        RwLockWriteGuard {
            lock: &read_guard.lock,
        }
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.readers.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> core::ops::Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.writer.store(0, Ordering::Release);
    }
}

impl<'a, T> core::ops::Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> core::ops::DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}
