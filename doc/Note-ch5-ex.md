## 5.EX 多核处理器支持（以k210为例）

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

2. 将UPSafeCell升级为线程安全的锁
    
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

### 1. 启动机制
芯片启动时，全部两个核心都会启动，但是只有一个核心会处于运行状态，另一个核心则处于待机状态。我们需要先在一个核心上完成全局初始化任务，然后将待机状态的核心唤醒，使其进入运行状态。之后两个核心各自完成自己的初始化任务，进入正常运行状态。

首先我们需要修改入口`_start`方法，使两个核心分别使用不同的启动栈：
```
    # os/src/entry.asm

        .section .text.entry
        .globl _start
    _start:
        # RustSBI会将处理器ID放入a0寄存器
        # 各内核启动栈栈顶的位置为：boot_stack_lower_bound + 64KiB * (处理器ID + 1)
        add t0, a0, 1                   # t0 = a0 + 1
        slli t0, t0, 16                 # K210 上每个启动栈大小为 64KiB （0x10000），所以这里将处理器ID左移 16 位
        la sp, boot_stack_lower_bound   # sp = boot_stack_lower_bound
        add sp, sp, t0                  # sp = sp + t0

        call rust_main                  # transfer control to kernel Func

        .section .bss.stack
        .globl boot_stack_lower_bound   # mark the stack lower bound
    boot_stack_lower_bound:
        .space 4096 * 16 * 2            # set the stack space as 4096*16Byte =  64KB
        .globl boot_stack_top           # mark the top position of the stack    when booting
    boot_stack_top:
```

然后，针对不同的核心，我们需要在`main.rs`中实现不同的初始化逻辑：

```rust
//! os/src/main.rs

// ...

const BOOT_HART: usize = 0;

#[no_mangle] //disable mangle of func name 'rust_main'
pub fn rust_main(hart_id: usize, _device_tree_paddr: usize) -> ! {
    cpu::set_cpu_id(hart_id);
    if hart_id == BOOT_HART {
        // 进行全局初始化
        global_init(_device_tree_paddr);
        // 启动其他核
        for i in 1..config::CPU_NUM {
            cpu::send_ipi(i);
        }
    }
    // 各核心初始化
    hart_init(hart_id);

    unreachable!("Unreachable in rust_main!");
}
```

其中global_init中要进行全局初始化操作，如初始化内存等，其实现如下：

```rust
/// 全局初始化，应由BOOT核完成
fn global_init(_device_tree_paddr: usize) {
    //初始化bss段
    init_bss();

    // 输出内核版本信息
    println!("");
    println!("< OrCore build: {} >", env!("CARGO_PKG_VERSION"),);

    // 初始化内存管理子模块
    println!("Initializing memory management...");
    mem::init();

    // 初始化日志子模块
    println!("Initializing log module...");
    kernel_log::init();

    // 初始化进程管理子模块
    task::add_initproc(); // 添加initproc任务

    loader::list_apps(); // 列出所有App

    info!("System init finished");
}
```

hart_init中要进行各核心的初始化操作，如初始化进程管理器等，其实现如下：
```rust
fn hart_init(hart_id: usize) {
    info!("Hello RISC-V! in hart {}", hart_id);

    unsafe {
        riscv::register::sstatus::set_sum();
    }

    // 初始化系统调用子模块
    info!("Init trap handler...");
    trap::init();

    // 启用内核空间
    mem::KERNEL_SPACE.clone().read().activate();

    // 初始化时钟中断
    info!("Init time interrupt...");
    trap::enable_timer_interrupt(); // 启用时钟中断
    util::time::set_next_timer(); // 设置下个时钟中断

    // kernel初始化完成，开始运行第一个任务
    info!("Hart init finished.");
    task::run_tasks(); // 运行任务

    unreachable!("Unreachable in other_hart_init!");
}
```

注意到，我们在`rust_main`中启动其它核心时使用了一个新的方法`cpu::send_ipi(i);`，它是RustSBI的send_ipi方法的封装，其实现如下：

> * `sbi_call::send_ipi`方法的参数hart_mask是一个usize类型的值，表示需要唤醒的核心的位掩码，其中每一位对应一个核心，1表示发送，0表示不发送。这里我们使用了usize::MAX，表示唤醒所有核心。
> * 还有一点，`sbi_call`调用ecall时，传入的是hart_mask的地址，而不是直接传入hart_mask的值。因此我们需要在封装时加入`&hart_mask as *const _ as usize`这样的转换。

```rust
//! os/src/util/cpu.rs

/// 向指定id的处理器核心发送中断请求，唤起核心
#[allow(unused)]
pub fn send_ipi(hart_id: usize) {
    let hart_mask = 1 << hart_id;
    sbi_call::send_ipi(hart_mask);
}

/// 向其它处理器核心发送中断请求，唤起所有核心
pub fn broadcast_ipi() {
    sbi_call::send_ipi(usize::MAX);
}

//! os/src/sbi_call.rs

/// send IPI to other harts
pub fn send_ipi(hart_mask: usize) {
    let hart_mask_addr = &hart_mask as *const _ as usize;
    sbi_call(SBI_SEND_IPI, 0, hart_mask_addr, 0, 0);
}

```

还注意到，我们在`rust_main`开头处调用了`cpu::set_cpu_id(hart_id);`，这是为了在后续的代码中可以获取当前核心的ID，其实现如下：

```rust
//! os/src/util/cpu.rs

/// 设置当前核心的ID
/// 
```

### 2. 多核同步

到上一节为止，我们的kernel已经可以启动k210上的两个核心了。但是由于两个核心异步运行，造成了很多问题。例如串口输出时，如果恰好两个核心都需要输出，则会出现串口输出混乱的情况。因此我们需要对核心之间的操作进行同步。

在这里，我们只对串口输出进行同步，其他的同步操作可以根据需要进行扩展。我们修改`os/src/console.rs`中的`print`方法，使其在输出时使用自旋锁进行锁定，保证串口输出的原子性。

```rust
//! os/src/console.rs

lazy_static! {
    static ref STDOUT_LOCK: SpinLock<()> = SpinLock::new(());
}

pub fn print(args: fmt::Arguments) {
    let _guard = STDOUT_LOCK.lock();
    Stdout.write_fmt(args).unwrap();
}
```

### 3. 进程调度

