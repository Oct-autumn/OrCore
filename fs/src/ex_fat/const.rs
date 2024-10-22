/// boot_jump标识
pub const EXFAT_BOOT_JUMP: [u8; 3] = [0xEB, 0x76, 0x90];
/// exFAT文件系统标识
pub const EXFAT_SIGNATURE: [u8; 8] = *b"EXFAT   ";
/// exFAT版本号（高位1，低位0）
pub const EXFAT_VERSION: [u8; 2] = [0x00, 0x01];
/// Boot结束符
pub const EXFAT_BOOT_SIGNATURE: [u8; 2] = [0x55, 0xAA];
pub const EXFAT_EXBOOT_SIGNATURE: [u8; 4] = [0x00, 0x00, 0x55, 0xAA];

/// 簇大小描述（8位）
pub const SECTORS_PER_CLUSTER_SHIFT: usize = 8;
/// 簇大小（128扇区，合64KB）
pub const SECTORS_PER_CLUSTER: usize = 1 << SECTORS_PER_CLUSTER_SHIFT;

/// 文件名长度上限
pub const EXFAT_MAX_FILE_LEN: usize = 255;
/// 时间戳最小值 Jan 1 GMT 00:00:00 1980
pub const EXFAT_MIN_TIMESTAMP_MSECS: u64 = 315532800000;
/// 时间戳最大值 Dec 31 GMT 23:59:59 2107
pub const EXFAT_MAX_TIMESTAMP_MSECS: u64 = 4354819199000;