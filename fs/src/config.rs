//! fs/src/config.rs
//!
//! 文件系统配置

use core::convert::Into;

// 扇区相关
/// 扇区大小描述（9位）
pub const BYTES_PER_SECTOR_SHIFT: usize = 9;
/// 扇区大小（512Byte）
pub const SECTOR_BYTES: usize = 1 << BYTES_PER_SECTOR_SHIFT;
/// 扇区位数（4096Bit）
pub const SECTOR_BITS: usize = SECTOR_BYTES * 8;

// 簇相关
/// 簇大小描述（8位）
pub const SECTORS_PER_CLUSTER_SHIFT: usize = 8;
/// 簇大小（128扇区，合64KB）
pub const SECTORS_PER_CLUSTER: usize = 1 << SECTORS_PER_CLUSTER_SHIFT;

// 块（扇区）缓存相关
/// 块（扇区）缓存最大大小（16块（扇区））
pub const BLOCK_CACHE_MAX_SIZE: usize = 16;

// exFAT相关
/// boot_jump标识
pub const EXFAT_BOOT_JUMP: [u8; 3] = [0xEB, 0x76, 0x90];
/// exFAT文件系统标识
pub const EXFAT_SIGNATURE: [u8; 8] = *b"EXFAT   ";
/// exFAT版本号（高位1，低位0）
pub const EXFAT_VERSION: u16 = 0x01_00;
/// Boot结束符
pub const EXFAT_BOOT_END: [u8; 2] = [0x55, 0xAA];
