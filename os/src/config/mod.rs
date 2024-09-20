pub const USER_STACK_SIZE: usize = 4096 * 2; // 用户栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 内核栈大小
pub const MAX_APP_NUM: usize = 16; // 最大App数量
pub const APP_BASE_ADDRESS: usize = 0x80400000; // 存放用户App的基地址
pub const APP_SIZE_LIMIT: usize = 0x20000; // 用户App大小限制

// 时钟频率（非CPU频率）
#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;
#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
pub const CLOCK_FREQ: usize = 12500000;

pub const TICKS_PER_SEC: usize = 100; // 每秒时钟中断次数（注意，如果内核日志输出等级过高，会导致用户程序因无法分得足够的时间片而不能正常运行）

// 内核堆大小（3MB）
#[cfg(feature = "board_k210")]
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
pub const KERNEL_HEAP_SIZE: usize = 0x20_0000;
