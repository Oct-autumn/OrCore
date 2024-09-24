pub const USER_STACK_SIZE: usize = 4096 * 2; // 用户栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 内核栈大小

// 时钟频率（非CPU频率）
#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;
#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
pub const CLOCK_FREQ: usize = 12500000;

pub const TICKS_PER_SEC: usize = 20; // 每秒时钟中断次数（注意，如果内核日志输出等级过高，会导致用户程序因无法分得足够的时间片而不能正常运行）

pub const MEMORY_END: usize = 0x80800000; // 内存结束地址

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000; // 内核堆大小（3MB）
pub const PAGE_SIZE_BITS: usize = 0xc; // 内存分页
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS; // 页大小
pub const PAGE_OFFSET_MASK: usize = PAGE_SIZE - 1; // 页偏移掩码

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1; // 用于存放中断处理汇编代码的地址
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE; // 存放TrapContext的地址

// 为app分配内核栈
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}
