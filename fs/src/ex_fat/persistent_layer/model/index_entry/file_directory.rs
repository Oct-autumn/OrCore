//! fs/src/ex_fat/persistent_layer/model/index_entry/file_directory.rs
//!
//! 文件目录项
//!
//! 时间戳

use bitflags::bitflags;
use chrono::{DateTime, Datelike, NaiveDate, Timelike};
use core::fmt::{Debug, Formatter};
use crate::ex_fat::persistent_layer::up_case_table::FileNameHash;
use super::super::{
    cluster_id::ClusterId,
    index_entry::{IndexEntryChecksum, IndexEntryCostumeBytes},
    unicode_str::UnicodeString
};

#[repr(C)]
#[derive(Debug, Clone)]
pub enum FileDirectoryCostume {
    Costume1(FileDirectoryCostume1),
    Costume2(FileDirectoryCostume2),
    Costume3(FileDirectoryCostume3),
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FileAttributes: u16 {
        const ReadOnly = 1 << 0;
        const Hidden = 1 << 1;
        const System = 1 << 2;
        const Directory = 1 << 4;
        const Archive = 1 << 5;
    }
}

impl FileAttributes {
    pub fn read_only(&mut self, read_only: bool) -> Self {
        self.set(FileAttributes::ReadOnly, read_only);
        *self
    }
    
    pub fn hidden(&mut self, hidden: bool) -> Self {
        self.set(FileAttributes::Hidden, hidden);
        *self
    }
    
    pub fn system(&mut self, system: bool) -> Self {
        self.set(FileAttributes::System, system);
        *self
    }
    
    pub fn directory(&mut self, directory: bool) -> Self {
        self.set(FileAttributes::Directory, directory);
        *self
    }
    
    pub fn archive(&mut self, archive: bool) -> Self {
        self.set(FileAttributes::Archive, archive);
        *self
    }
}


/// 属性1
///
/// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
/// | :------- | :--------------- | :--------- |
/// | 0x01     | 1                | 附属目录项数 |
/// | 0x02     | 2                | 校验和 |
/// | 0x04     | 2                | 文件属性 |
/// | 0x06     | 2                | 保留 |
/// | 0x08     | 4                | 创建时间 |
/// | 0x0C     | 4                | 最后修改时间 |
/// | 0x10     | 4                | 最后访问时间 |
/// | 0x14     | 1                | 文件创建时间精确至10ms |
/// | 0x15     | 11               | 保留 |
#[derive(Clone)]
pub struct FileDirectoryCostume1 {
    pub secondary_count: u8,
    pub set_check_sum: IndexEntryChecksum,
    pub file_attributes: FileAttributes,
    pub create_time_stamp: TimeStamp,
    pub last_modified_time_stamp: TimeStamp,
    pub last_accessed_time_stamp: TimeStamp,
    pub create_10ms_increment: u8,
}

impl FileDirectoryCostume1 {
    pub fn new(
        secondary_count: u8,
        set_check_sum: IndexEntryChecksum,
        file_attributes: FileAttributes,
        create_time_stamp: TimeStamp,
        last_modified_time_stamp: TimeStamp,
        last_accessed_time_stamp: TimeStamp,
        create_10ms_increment: u8,
    ) -> Self {
        Self {
            secondary_count,
            set_check_sum,
            file_attributes,
            create_time_stamp,
            last_modified_time_stamp,
            last_accessed_time_stamp,
            create_10ms_increment,
        }
    }
}

impl IndexEntryCostumeBytes for FileDirectoryCostume1 {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.secondary_count;
        arr[1..3].copy_from_slice(&self.set_check_sum.0.to_le_bytes());
        arr[3..5].copy_from_slice(&self.file_attributes.bits().to_le_bytes());
        arr[8..12].copy_from_slice(&self.create_time_stamp.0.to_le_bytes());
        arr[12..16].copy_from_slice(&self.last_modified_time_stamp.0.to_le_bytes());
        arr[16..20].copy_from_slice(&self.last_accessed_time_stamp.0.to_le_bytes());
        arr[20] = self.create_10ms_increment;

        arr
    }

    fn from_bytes(arr: &[u8]) -> Self {
        assert_eq!(arr.len(), 31);
        let set_check_sum = IndexEntryChecksum(<u16>::from_le_bytes([arr[1], arr[2]]));
        let file_attributes = FileAttributes::from_bits(<u16>::from_le_bytes([arr[3], arr[4]])).unwrap();
        let create_time_stamp = <u32>::from_le_bytes([arr[8], arr[9], arr[10], arr[11]]);
        let last_modified_time_stamp = <u32>::from_le_bytes([arr[12], arr[13], arr[14], arr[15]]);
        let last_accessed_time_stamp = <u32>::from_le_bytes([arr[16], arr[17], arr[18], arr[19]]);

        Self {
            secondary_count: arr[0],
            set_check_sum,
            file_attributes,
            create_time_stamp: TimeStamp(create_time_stamp),
            last_modified_time_stamp: TimeStamp(last_modified_time_stamp),
            last_accessed_time_stamp: TimeStamp(last_accessed_time_stamp),
            create_10ms_increment: arr[20],
        }
    }
}

impl Debug for FileDirectoryCostume1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileDirectory1Costume")
            .field("secondary_count", &self.secondary_count)
            .field("set_check_sum", &self.set_check_sum)
            .field("file_attributes", &self.file_attributes)
            .field("create_time_stamp", &self.create_time_stamp)
            .field("last_modified_time_stamp", &self.last_modified_time_stamp)
            .field("last_accessed_time_stamp", &self.last_accessed_time_stamp)
            .field("create_10ms_increment", &self.create_10ms_increment)
            .finish()
    }
}

bitflags! {
    /// 文件碎片标志
    #[derive(Copy, Clone)]
    pub struct FragmentFlag: u8 {
        const Continuous = 0x03;
        const Fragmented = 0x01;
    }
}

impl Debug for FragmentFlag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.contains(FragmentFlag::Continuous) {
            write!(f, "CONTINUOUS")
        } else if self.contains(FragmentFlag::Fragmented) {
            write!(f, "FRAGMENTED")
        } else {
            write!(f, "UNKNOWN")
        }
    }
}

/// 属性2
///
/// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
/// | :------- | :--------------- | :--------- |
/// | 0x01     | 1                | 文件碎片标志（连续存放（无碎片）为“03H”，非连续存放（有碎片）为“01H”） |
/// | 0x02     | 1                | 保留 |
/// | 0x03     | 1                | 文件名字符数 |
/// | 0x04     | 2                | 文件名哈希值 |
/// | 0x06     | 2                | 保留 |
/// | 0x08     | 8                | 文件大小1 |
/// | 0x10     | 4                | 保留 |
/// | 0x14     | 4                | 起始簇号 |
/// | 0x18     | 8                | 文件大小2 |
#[derive(Clone)]
pub struct FileDirectoryCostume2 {
    pub fragment_flag: FragmentFlag,
    pub file_name_length: u8,
    pub file_name_hash: FileNameHash,
    pub file_size1: u64,
    pub start_cluster: ClusterId,
    pub file_size2: u64,
}

impl FileDirectoryCostume2 {
    pub fn new(
        fragment_flag: FragmentFlag,
        file_name_length: u8,
        file_name_hash: FileNameHash,
        file_size1: u64,
        start_cluster: ClusterId,
        file_size2: u64,
    ) -> Self {
        Self {
            fragment_flag,
            file_name_length,
            file_name_hash,
            file_size1,
            start_cluster,
            file_size2,
        }
    }
}

impl IndexEntryCostumeBytes for FileDirectoryCostume2 {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.fragment_flag.bits();
        arr[2] = self.file_name_length;
        arr[3..5].copy_from_slice(&self.file_name_hash.0.to_le_bytes());
        arr[7..15].copy_from_slice(&self.file_size1.to_le_bytes());
        arr[19..23].copy_from_slice(&self.start_cluster.0.to_le_bytes());
        arr[23..31].copy_from_slice(&self.file_size2.to_le_bytes());

        arr
    }

    fn from_bytes(arr: &[u8]) -> Self {
        assert_eq!(arr.len(), 31);
        let fragment_flag = FragmentFlag::from_bits(arr[0]).unwrap();
        let file_name_hash = FileNameHash(<u16>::from_le_bytes([arr[3], arr[4]]));
        let file_size1 = <u64>::from_le_bytes([arr[7], arr[8], arr[9], arr[10], arr[11], arr[12], arr[13], arr[14]]);
        let start_cluster = <u32>::from_le_bytes([arr[19], arr[20], arr[21], arr[22]]);
        let file_size2 = <u64>::from_le_bytes([arr[23], arr[24], arr[25], arr[26], arr[27], arr[28], arr[29], arr[30]]);

        Self {
            fragment_flag,
            file_name_length: arr[2],
            file_name_hash,
            file_size1,
            start_cluster: ClusterId(start_cluster),
            file_size2,
        }
    }
}

impl Debug for FileDirectoryCostume2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileDirectory2Costume")
            .field("fragment_flag", &self.fragment_flag)
            .field("file_name_length", &self.file_name_length)
            .field("file_name_hash", &self.file_name_hash)
            .field("file_size1", &self.file_size1)
            .field("start_cluster", &self.start_cluster)
            .field("file_size2", &self.file_size2)
            .finish()
    }
}

/// 属性3
///
/// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
/// | :------- | :--------------- | :--------- |
/// | 0x01     | 1                | 保留 |
/// | 0x02     | 2N               | 文件名 |
#[derive(Clone)]
pub struct FileDirectoryCostume3 {
    pub file_name: UnicodeString,
}

impl FileDirectoryCostume3 {
    pub fn new(file_name: UnicodeString) -> Self {
        Self {
            file_name,
        }
    }
}

impl IndexEntryCostumeBytes for FileDirectoryCostume3 {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        let bytes = self.file_name.to_le_bytes();
        
        arr[1..1 + bytes.len()].copy_from_slice(bytes.as_slice());

        arr
    }

    fn from_bytes(arr: &[u8]) -> Self {
        assert_eq!(arr.len(), 31);
        Self {
            file_name: UnicodeString::from_le_bytes(arr[1..].as_ref()),
        }
    }
}
impl Debug for FileDirectoryCostume3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileDirectory3Costume")
            .field("file_name", &self.file_name.to_string())
            .finish()
    }
}

/// 时间戳
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimeStamp(pub u32);

impl TimeStamp {
    pub fn get_year(&self) -> u16 {
        ((self.0 >> 25) & 0x7F) as u16 + 1980
    }

    pub fn get_month(&self) -> u8 {
        ((self.0 >> 21) & 0x0F) as u8
    }

    pub fn get_day(&self) -> u8 {
        ((self.0 >> 16) & 0x1F) as u8
    }

    pub fn get_hour(&self) -> u8 {
        ((self.0 >> 11) & 0x1F) as u8
    }

    pub fn get_minute(&self) -> u8 {
        ((self.0 >> 5) & 0x3F) as u8
    }

    pub fn get_second(&self) -> u8 {
        ((self.0 & 0x1F) * 2) as u8
    }

    pub fn set_year(&mut self, year: u16) {
        self.0 = (self.0 & 0x01FFFE00) | (((year - 1980) as u32) << 25);
    }

    pub fn set_month(&mut self, month: u8) {
        self.0 = (self.0 & 0x01FFE1FF) | ((month as u32) << 21);
    }

    pub fn set_day(&mut self, day: u8) {
        self.0 = (self.0 & 0x01FFFE00) | ((day as u32) << 16);
    }

    pub fn set_hour(&mut self, hour: u8) {
        self.0 = (self.0 & 0x01FFFE00) | ((hour as u32) << 11);
    }

    pub fn set_minute(&mut self, minute: u8) {
        self.0 = (self.0 & 0x01FFFE00) | ((minute as u32) << 5);
    }

    pub fn set_second(&mut self, second: u8) {
        self.0 = (self.0 & 0x01FFE0FF) | ((second >> 1) as u32);
    }

    pub fn from_unix_ms_timestamp(ms_timestamp: u64) -> (Self, u8) {
        (Self::from_unix_timestamp(ms_timestamp / 1000), ((ms_timestamp % 2000) / 10) as u8)
    }

    pub fn from_unix_timestamp(timestamp: u64) -> Self {
        let date_time = DateTime::from_timestamp(timestamp as i64, 0).unwrap();
        let mut ret = Self(0);

        ret.set_second(date_time.second() as u8);
        ret.set_minute(date_time.minute() as u8);
        ret.set_hour(date_time.hour() as u8);
        ret.set_day(date_time.day() as u8);
        ret.set_month(date_time.month() as u8);
        ret.set_year(date_time.year() as u16);

        ret
    }

    pub fn to_unix_timestamp(&self) -> u64 {
        NaiveDate::from_ymd_opt(
            self.get_year() as i32,
            self.get_month() as u32,
            self.get_day() as u32,
        ).unwrap().and_hms_opt(
            self.get_hour() as u32,
            self.get_minute() as u32,
            self.get_second() as u32,
        ).unwrap().and_utc()
            .timestamp() as u64
    }
}