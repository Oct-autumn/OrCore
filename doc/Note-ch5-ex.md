## 5.EX 多核处理器支持

k210有两个核心，如果只用一个核心的话太浪费了。所以，本小节我们将实现OS对多核处理器的支持。

### 0. 前期准备

1. 改进锁机制

    首先我们需要修改`os\stc\sync`模块，实现一个`SpinLock`自旋锁。
    
    ```rust
    //! os/src/sync/spin_lock.rs
    //! 自旋锁

    use core::{cell::UnsafeCell, sync::atomic::AtomicBool};

    pub struct SpinLock<T> {
        /// 原子锁（0 未锁，1 锁定）
        locked: AtomicBool,

        /// 内部可变性
        inner: UnsafeCell<T>,
    }

    unsafe impl<T> Sync for SpinLock<T> {}

    impl<T> SpinLock<T> {
        pub fn new(value: T) -> Self {
            Self {
                locked: AtomicBool::new(false),
                inner: UnsafeCell::new(value),
            }
        }

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
            {
                // 自旋等待锁释放
            }
            SpinLockGuard { lock: self }
        }
    }

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

    再实现一个`RwLock`读写锁。

    ```rust
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
    ```

1. 将UPSafeCell升级为线程安全的锁
    
    根据使用环境的不同，我们要将原来使用UPSafeCell的地方改为使用不同的线程安全的锁：
      - 对于很少出现多读的情况，我们使用自旋锁，减少开销。
      - 对于经常出现多读的情况，我们使用读写锁，提高并发性能。

    被修改为自旋锁的地方包括：
      - 物理页帧分配器`FRAME_ALLOCATOR`
      - 进程管理器`PROCESS_MANAGER`
      - PID分配器`PID_ALLOCATOR`
    
    被修改为读写锁的地方包括：
      - 内核栈`KERNEL_SPACE`
      - 处理机实例`PROCESSOR`
      - PCB内部字段`ProcessControlBlock::inner`
