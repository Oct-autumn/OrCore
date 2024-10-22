//! fs/ex_fat/cluster_chain/boot_sector
//!
//! DBR相关结构体定义：MBR/BBR/BR校验和

use bitflags::bitflags;

use crate::config;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct VolumeFlags: u16 {
        /// **启用的FAT表**
        const ActiveFat = 1 << 0;
        /// **是否为脏卷**
        const VolumeDirty = 1 << 1;
        /// **存储介质故障**
        const MediaFailure = 1 << 2;
    }
}

/// **DBR块**
///
/// 用于存放文件系统的根信息
///
/// - FAT表扇区号：fat_offset
/// - 簇位图扇区号：cluster_heap_offset
/// - 大写字符扇区号：cluster_heap_offset + (1 << sectors_per_cluster_shift) * 1
/// - 根目录扇区号：cluster_heap_offset + (1 << sectors_per_cluster_shift) * (first_cluster_of_root_directory - 2)
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BootSector {
    /// **跳转字段** 0x000 3B
    pub jump_boot: [u8; 3],
    /// **文件系统名** 0x003 8B
    pub file_system_name: [u8; 8],
    /// **对齐** 0x00B 53B
    must_be_zero: [u8; 53],
    /// **卷偏移量**（单位：扇区，为0时应忽略） 0x040 8B
    pub partition_offset: u64,
    /// **卷大小**（单位：扇区） 0x048 8B
    pub volume_length: u64,
    /// **FAT表偏移量**（单位：扇区） 0x050 4B
    pub fat_offset: u32,
    /// **FAT表长度**（单位：扇区） 0x054 4B
    pub fat_length: u32,
    /// **首簇偏移量**（单位：扇区） 0x058 4B
    pub cluster_heap_offset: u32,
    /// **卷内簇数量** 0x05C 4B
    pub cluster_count: u32,
    /// **根目录起始簇号** 0x060 4B
    pub first_cluster_of_root_directory: u32,
    /// **卷序列号（用于区分不同卷）** 0x064 4B
    pub volume_serial_number: u32,
    /// **文件系统版本号** 0x068 2B
    pub filesystem_revision: [u8; 2],
    /// **卷状态** 0x06A 2B
    pub volume_flags: VolumeFlags,
    /// **每扇区字节数描述**（`2^N`字节） 0x06C 1B
    pub bytes_per_sector_shift: u8,
    /// **每簇扇区数描述**（`2^N`扇区） 0x06D 1B
    pub sectors_per_cluster_shift: u8,
    /// **FAT表个数** 0x06E 1B
    pub number_of_fats: u8,
    /// **驱动标记** 0x06F 1B
    pub drive_select: u8,
    /// **分区使用百分比** 0x070 1B
    pub percent_in_use: u8,
    /// **保留区域** 0x071 7B
    _reserved: [u8; 7],
    /// **启动代码区** 0x078 390B
    pub boot_code: [u8; 390],
    /// **结束符**（固定为0x55AA） 0x1FE 2B
    pub boot_signature: [u8; 2],
}

impl BootSector {
    pub fn create(
        volume_length: u64,
        fat_offset: u32,
        fat_length: u32,
        cluster_heap_offset: u32,
        cluster_count: u32,
        first_cluster_of_root_directory: u32,
        volume_serial_number: u32,
        bytes_per_sector: u32,
        sector_per_cluster: u32,
    ) -> Self {
        Self {
            jump_boot: crate::ex_fat::r#const::EXFAT_BOOT_JUMP,
            file_system_name: crate::ex_fat::r#const::EXFAT_SIGNATURE,
            must_be_zero: [0; 53],
            partition_offset: 0,
            volume_length,
            fat_offset,
            fat_length,
            cluster_heap_offset,
            cluster_count,
            first_cluster_of_root_directory,
            volume_serial_number,
            filesystem_revision: crate::ex_fat::r#const::EXFAT_VERSION,
            volume_flags: VolumeFlags::empty(),
            bytes_per_sector_shift: bytes_per_sector.trailing_zeros() as u8,
            sectors_per_cluster_shift: sector_per_cluster.trailing_zeros() as u8,
            number_of_fats: 1,
            drive_select: 0,
            percent_in_use: 0,
            _reserved: [0; 7],
            boot_code: [0; 390],
            boot_signature: crate::ex_fat::r#const::EXFAT_BOOT_SIGNATURE,
        }
    }

    /// 检查是否为exFAT文件系统
    pub fn is_exfat(&self) -> bool {
        self.jump_boot == crate::ex_fat::r#const::EXFAT_BOOT_JUMP && self.file_system_name == crate::ex_fat::r#const::EXFAT_SIGNATURE
    }

    /// 卷状态
    pub fn volume_flags(&self) -> VolumeFlags {
        self.volume_flags
    }

    /// 有效性检查
    pub fn check_valid(&self) -> Result<(), String> {
        if self.boot_signature != crate::ex_fat::r#const::EXFAT_BOOT_SIGNATURE {
            Err(format!("Invalid boot record signature: {:?}", self.boot_signature))
        } else if self.file_system_name != crate::ex_fat::r#const::EXFAT_SIGNATURE {
            Err(format!("Invalid fs_name: {:?}", self.file_system_name))
        } else if self.must_be_zero.iter().find(|byte| **byte != 0).is_some() {
            Err(format!("Invalid must_be_zero: {:?}", self.must_be_zero))
        } else if self.number_of_fats != 1 {
            Err(format!("Unsupported number of fats: {:?}", self.number_of_fats))
        } else if self.bytes_per_sector_shift != 9 {
            Err(format!("Unsupported sector size: {:?}", self.bytes_per_sector_shift))
        } else if self.sectors_per_cluster_shift > 16 {
            Err(format!("Sectors per cluster shift too large: {:?}", self.sectors_per_cluster_shift))
        } else if (self.fat_length << self.bytes_per_sector_shift) < ((self.cluster_count + 2) << 2) {
            Err(format!("Invalid fat length: {:?}", self.fat_length))
        } else if self.cluster_heap_offset < (self.fat_offset + self.fat_length) {
            Err(format!("Invalid cluster heap offset: {:?}", self.cluster_heap_offset))
        } else if self.volume_flags.contains(VolumeFlags::VolumeDirty) {
            Err("Volume was not properly unmounted. Some data may be corrupt. Please run fsck.".to_string())
        } else if self.volume_flags.contains(VolumeFlags::MediaFailure) {
            Err("Medium has reported failures. Some data may be lost.".to_string())
        } else {
            Ok(())
        }
    }

    pub fn to_bytes(&self) -> [u8; 512] {
        unsafe { core::mem::transmute(*self) }
    }

    pub fn from_bytes(bytes: [u8; 512]) -> Self {
        unsafe { core::mem::transmute(bytes) }
    }
}

/// **引导区校验和**
#[derive(Default, Debug, Eq, PartialEq)]
pub struct BootChecksum(pub u32);

impl BootChecksum {
    /// 读取扇区，计算校验和
    pub fn add_sector(&mut self, sector: &[u8], is_boot_sector: bool) {
        assert_eq!(sector.len(), config::SECTOR_BYTES); // 输入的slice大小必须为一个扇区
        let number_of_bytes: u32 = config::SECTOR_BYTES as u32;
        let mut checksum: u32 = self.0;

        for index in 0..number_of_bytes {
            if is_boot_sector && (index == 106 || index == 107 || index == 112) {
                continue;
            } else {
                checksum = ((checksum << 31) | (checksum >> 1)).wrapping_add(sector[index as usize] as u32);
            }
        }
        self.0 = checksum;
    }
}
