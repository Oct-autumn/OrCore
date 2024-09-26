## 5.EX 多核处理器支持

k210有两个核心，如果只用一个核心的话太浪费了。所以，本小节我们将实现OS对多核处理器的支持。

### 前期准备

首先我们需要修改`os\stc\sync`模块，实现一个SpinLock。

```rust
//! os/src/sync/ups_cell.rs
//! 自旋锁

use core::{cell::UnsafeCell, sync::atomic::AtomicBool};

/// 自旋锁
pub struct SpinLock<T> {
    /// 原子锁芯
    locked: AtomicBool,
    /// 内部可变性
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// 创建一个新的自旋锁
    pub unsafe fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            inner: UnsafeCell::new(value),
        }
    }

    /// 获取锁
    pub fn lock(&self) -> SpinLockGuard<T> {
        while self
            .locked
            .compare_exchange(
                false,
                true,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {}
        SpinLockGuard { lock: self }
    }
}

/// 自旋锁守卫
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock
            .locked
            .store(false, core::sync::atomic::Ordering::Release);
    }
}

impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.inner.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.inner.get() }
    }
}
```