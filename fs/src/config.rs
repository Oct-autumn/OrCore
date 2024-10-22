//! fs/src/config.rs
//!
//! 文件系统配置

// 扇区相关
/// 扇区大小描述（9位）
pub const BYTES_PER_SECTOR_SHIFT: usize = 9;
/// 扇区大小（512Byte）
pub const SECTOR_BYTES: usize = 1 << BYTES_PER_SECTOR_SHIFT;
/// 扇区位数（4096Bit）
pub const SECTOR_BITS: usize = SECTOR_BYTES * 8;
