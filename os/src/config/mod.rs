pub const USER_STACK_SIZE: usize = 4096 * 2; // 用户栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 内核栈大小

// 时钟频率（非CPU频率）
#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;
#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
pub const CLOCK_FREQ: usize = 12500000;

pub const TICKS_PER_SEC: usize = 10; // 每秒时钟中断次数（注意，如果内核日志输出等级过高，会导致用户程序因无法分得足够的时间片而不能正常运行）

pub const MEMORY_END: usize = 0x80800000; // 内存结束地址

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000; // 内核堆大小（3MB）
pub const PAGE_SIZE_BITS: usize = 0xc; // 内存分页
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS; // 页大小
pub const PAGE_OFFSET_MASK: usize = PAGE_SIZE - 1; // 页偏移掩码

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1; // 用于存放中断处理汇编代码的地址
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE; // 存放TrapContext的地址

// 为避免分配相邻PID导致误认为是同一进程的问题，维持PID回收队列中最少的PID数量
pub const MIN_PID_RECYCLE: usize = 10;

pub const CPU_NUM: usize = 2; // CPU数量

#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
pub const MMIO: &[(usize, usize)] = &[(0x10000000, 0x10000)];

#[cfg(feature = "board_k210")]
pub const MMIO: &[(usize, usize)] = &[
    // we don't need clint in S priv when running
    // we only need claim/complete for target0 after initializing
    (0x0C00_0000, 0x3000), /* PLIC      */
    (0x0C20_0000, 0x1000), /* PLIC      */
    (0x3800_0000, 0x1000), /* UARTHS    */
    (0x3800_1000, 0x1000), /* GPIOHS    */
    (0x5020_0000, 0x1000), /* GPIO      */
    (0x5024_0000, 0x1000), /* SPI_SLAVE */
    (0x502B_0000, 0x1000), /* FPIOA     */
    (0x502D_0000, 0x1000), /* TIMER0    */
    (0x502E_0000, 0x1000), /* TIMER1    */
    (0x502F_0000, 0x1000), /* TIMER2    */
    (0x5044_0000, 0x1000), /* SYSCTL    */
    (0x5200_0000, 0x1000), /* SPI0      */
    (0x5300_0000, 0x1000), /* SPI1      */
    (0x5400_0000, 0x1000), /* SPI2      */
];
