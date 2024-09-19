pub const USER_STACK_SIZE: usize = 4096 * 2; // 用户栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 内核栈大小
pub const MAX_APP_NUM: usize = 16; // 最大App数量
pub const APP_BASE_ADDRESS: usize = 0x80400000; // 存放用户App的基地址
pub const APP_SIZE_LIMIT: usize = 0x20000; // 用户App大小限制
