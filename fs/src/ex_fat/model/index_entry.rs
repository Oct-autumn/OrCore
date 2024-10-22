//! fs/src/ex_fat/cluster_chain/model/index_entry/mod.rs
//!
//! exFAT目录项

use super::{
    cluster_id::ClusterId,
    unicode_str::UnicodeString,
};
use crate::ex_fat::model::up_case_table::FileNameHash;
use alloc::vec::Vec;
use bitflags::bitflags;
use chrono::{DateTime, Datelike, NaiveDate, Timelike};
use core::fmt::Debug;
use std::ptr;
use std::ptr::{addr_of, read_unaligned};

pub trait IndexEntryCostumeBytes {
    fn to_bytes(&self) -> [u8; 31];
    fn from_bytes(bytes: &[u8]) -> Self;
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct IndexEntryType: u8 {
        /// 未使用（目录结束标志）
        const ExfatUnused       = 0x00;
        /// 无效值
        const ExfatInval        = 0x80;
        /// 簇分配位图
        const ExfatBitmap       = 0x81;
        /// 大写字母表
        const ExfatUpcase       = 0x82;
        /// 卷标
        const ExfatVolume       = 0x83;
        /// 文件或目录
        const ExfatFile         = 0x85;
        /// GUID
        const ExfatGUID         = 0xA0;
        /// 填充
        const ExfatPadding      = 0xA1;
        /// ACL TAB
        const ExfatACLTab       = 0xA2;
        /// 流扩展
        const ExfatStream       = 0xC0;
        /// 文件名
        const ExfatName         = 0xC1;
        /// ACL
        const ExfatACL          = 0xC2;
        /// Vendor extension
        const ExfatVendorExt    = 0xE0;
        /// Vendor allocation
        const ExfatVendorAlloc  = 0xE1;
    }
}

impl IndexEntryType {
    pub fn deleted(&mut self, deleted: bool) {
        self.set(IndexEntryType::ExfatInval, !deleted);
    }

    pub fn is_exfat_deleted(&self) -> bool { self.bits() < 0x80 }
    pub fn is_exfat_critical_pri(&self) -> bool { self.bits() < 0xA0 }
    pub fn is_exfat_benign_pri(&self) -> bool { self.bits() < 0xC0 }
    pub fn is_exfat_critical_sec(&self) -> bool { self.bits() < 0xE0 }
}


bitflags! {
    #[derive(Debug, Default, Copy, Clone)]
    pub struct Attributes: u16 {
        const ReadOnly = 1 << 0;
        const Hidden = 1 << 1;
        const System = 1 << 2;
        const Directory = 1 << 4;
        const Archive = 1 << 5;
    }
}

impl Attributes {
    pub fn read_only(&mut self, read_only: bool) -> Self {
        self.set(Attributes::ReadOnly, read_only);
        *self
    }

    pub fn hidden(&mut self, hidden: bool) -> Self {
        self.set(Attributes::Hidden, hidden);
        *self
    }

    pub fn system(&mut self, system: bool) -> Self {
        self.set(Attributes::System, system);
        *self
    }

    pub fn directory(&mut self, directory: bool) -> Self {
        self.set(Attributes::Directory, directory);
        *self
    }

    pub fn archive(&mut self, archive: bool) -> Self {
        self.set(Attributes::Archive, archive);
        *self
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct FileCustom {
    pub secondary_count: u8,
    pub check_sum: IndexEntryChecksum,
    pub file_attributes: Attributes,
    _reserved1: [u8; 2],
    pub create_time_stamp: TimeStamp,
    pub last_modified_time_stamp: TimeStamp,
    pub last_accessed_time_stamp: TimeStamp,
    pub create_10ms_increment: u8,
    pub modify_10ms_increment: u8,
    pub create_tz: u8,
    pub modify_tz: u8,
    pub access_tz: u8,
    _reserved2: [u8; 7],
}

impl FileCustom {
    pub fn new(
        secondary_count: u8,
        check_sum: IndexEntryChecksum,
        file_attributes: Attributes,
        create_time_stamp: TimeStamp,
        last_modified_time_stamp: TimeStamp,
        last_accessed_time_stamp: TimeStamp,
        create_10ms_increment: u8,
        modify_10ms_increment: u8,
        create_tz: u8,
        modify_tz: u8,
        access_tz: u8,
    ) -> Self {
        Self {
            secondary_count,
            check_sum,
            file_attributes,
            _reserved1: [0u8; 2],
            create_time_stamp,
            last_modified_time_stamp,
            last_accessed_time_stamp,
            create_10ms_increment,
            modify_10ms_increment,
            create_tz,
            modify_tz,
            access_tz,
            _reserved2: [0u8; 7],
        }
    }
}

impl IndexEntryCostumeBytes for FileCustom {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.secondary_count;
        arr[1..3].copy_from_slice(&self.check_sum.0.to_le_bytes());
        arr[3..5].copy_from_slice((unsafe { read_unaligned(addr_of!(self.file_attributes)) })
            .bits().to_le_bytes().as_ref());
        arr[8..12].copy_from_slice(&self.create_time_stamp.0.to_le_bytes());
        arr[12..16].copy_from_slice(&self.last_modified_time_stamp.0.to_le_bytes());
        arr[16..20].copy_from_slice(&self.last_accessed_time_stamp.0.to_le_bytes());
        arr[20] = self.create_10ms_increment;
        arr[21] = self.modify_10ms_increment;
        arr[22] = self.create_tz;
        arr[23] = self.modify_tz;
        arr[24] = self.access_tz;

        arr
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let check_sum = IndexEntryChecksum(<u16>::from_le_bytes([bytes[1], bytes[2]]));
        let file_attributes = Attributes::from_bits(<u16>::from_le_bytes([bytes[3], bytes[4]])).unwrap();
        let create_time_stamp = <u32>::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let last_modified_time_stamp = <u32>::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let last_accessed_time_stamp = <u32>::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        Self {
            secondary_count: bytes[0],
            check_sum,
            file_attributes,
            _reserved1: [0u8; 2],
            create_time_stamp: TimeStamp(create_time_stamp),
            last_modified_time_stamp: TimeStamp(last_modified_time_stamp),
            last_accessed_time_stamp: TimeStamp(last_accessed_time_stamp),
            create_10ms_increment: bytes[20],
            modify_10ms_increment: bytes[21],
            create_tz: bytes[22],
            modify_tz: bytes[23],
            access_tz: bytes[24],
            _reserved2: [0u8; 7],
        }
    }
}

bitflags! {
    /// 文件碎片标志
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct FragmentFlag: u8 {
        const Continuous = 0x03;
        const Fragmented = 0x01;
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct StreamCustom {
    pub fragment_flag: FragmentFlag,
    _reserved1: u8,
    pub file_name_length: u8,
    pub file_name_hash: FileNameHash,
    _reserved2: [u8; 2],
    pub file_size1: u64,
    _reserved3: [u8; 4],
    pub start_cluster: ClusterId,
    pub file_size2: u64,
}

impl StreamCustom {
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
            _reserved1: 0u8,
            file_name_length,
            file_name_hash,
            _reserved2: [0u8, 2],
            file_size1,
            _reserved3: [0u8; 4],
            start_cluster,
            file_size2,
        }
    }
}

impl IndexEntryCostumeBytes for StreamCustom {
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

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let fragment_flag = FragmentFlag::from_bits(bytes[0]).unwrap();
        let file_name_hash = FileNameHash(<u16>::from_le_bytes([bytes[3], bytes[4]]));
        let file_size1 = <u64>::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14]]);
        let start_cluster = <u32>::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]);
        let file_size2 = <u64>::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30]]);

        Self {
            fragment_flag,
            _reserved1: 0u8,
            file_name_length: bytes[2],
            file_name_hash,
            _reserved2: [0u8, 2],
            file_size1,
            _reserved3: [0u8; 4],
            start_cluster: ClusterId(start_cluster),
            file_size2,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct NameCustom {
    pub flags: u8,
    pub name: [u16; 15],
}

impl NameCustom {
    pub fn new(name: [u16; 15]) -> Self {
        Self {
            flags: 0,
            name,
        }
    }
}

impl IndexEntryCostumeBytes for NameCustom {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.flags;
        for i in 0..15 {
            arr[2 * i + 1] = (self.name[i] & 0xFF) as u8;
            arr[2 * i + 2] = (self.name[i] >> 8) as u8;
        }

        arr
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let mut name = [0u16; 15];
        for i in 0..15 {
            name[i] = <u16>::from_le_bytes([bytes[2 * i + 1], bytes[2 * i + 2]]);
        }

        Self {
            flags: bytes[0],
            name,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct BitmapCustom {
    pub flags: u8,
    _reserved: [u8; 18],
    pub start_clu: u32,
    pub size: u64,
}

impl IndexEntryCostumeBytes for BitmapCustom {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.flags;
        arr[19..23].copy_from_slice(&self.start_clu.to_le_bytes());
        arr[23..31].copy_from_slice(&self.size.to_le_bytes());

        arr
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let start_clu = <u32>::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]);
        let size = <u64>::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30]]);

        Self {
            flags: bytes[0],
            _reserved: [0u8; 18],
            start_clu,
            size,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct UpCaseCustom {
    _reserved1: [u8; 3],
    pub checksum: u32,
    _reserved2: [u8; 12],
    pub start_clu: u32,
    pub size: u64,
}

impl IndexEntryCostumeBytes for UpCaseCustom {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[3..7].copy_from_slice(&self.checksum.to_le_bytes());
        arr[19..23].copy_from_slice(&self.start_clu.to_le_bytes());
        arr[23..31].copy_from_slice(&self.size.to_le_bytes());

        arr
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let checksum = <u32>::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
        let start_clu = <u32>::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]);
        let size = <u64>::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30]]);

        Self {
            _reserved1: [0u8; 3],
            checksum,
            _reserved2: [0u8; 12],
            start_clu,
            size,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct GenericCustom {
    pub flags: u8,
    pub custom_defined: [u8; 18],
    pub start_clu: u32,
    pub size: u64,
}

impl IndexEntryCostumeBytes for GenericCustom {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];

        arr[0] = self.flags;
        arr[1..19].copy_from_slice(&self.custom_defined);
        arr[19..23].copy_from_slice(&self.start_clu.to_le_bytes());
        arr[23..31].copy_from_slice(&self.size.to_le_bytes());

        arr
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 31);
        let mut custom_defined = [0u8; 18];
        custom_defined.copy_from_slice(&bytes[1..19]);
        let start_clu = <u32>::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]);
        let size = <u64>::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30]]);

        Self {
            flags: bytes[0],
            custom_defined,
            start_clu,
            size,
        }
    }
}

/// 目录项的派生项自定义部分
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub union EntryCustom {
    pub file: FileCustom,
    pub stream: StreamCustom,
    pub name: NameCustom,
    pub bitmap: BitmapCustom,
    pub up_case: UpCaseCustom,
    pub generic: GenericCustom,
    pub raw: [u8; 31],
}

impl Default for EntryCustom {
    fn default() -> Self {
        Self {
            raw: [0; 31],
        }
    }
}

impl Debug for EntryCustom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe {
            // 以16进制输出每个字节
            Ok(for i in 0..31 {
                write!(f, "{:02X} ", self.raw[i])?;
            })
        }
    }
}

/// 目录项
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct IndexEntry {
    /// 目录项类型
    pub entry_type: IndexEntryType,
    /// 本条目由派生的目录项定义
    pub custom_defined: EntryCustom,
}

impl IndexEntry {
    // 创建一个新的`卷标`目录项
    //
    // 卷标使用Unicode字符集，每个字符占用2个字节，最多15个字符（30字节）
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------: | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“83H”） |
    // | 0x01     | 1                | 卷标字符数 |
    // | 0x02     | 22               | 卷标 |
    // | 0x18     | 8                | 保留（也可用） |

    // 创建一个新的`簇分配位图`目录项
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------: | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“81H”） |
    // | 0x01     | 19               | 保留 |
    // | 0x14     | 4                | 起始簇号 |
    // | 0x18     | 8                | 文件大小 |

    // 创建一个新的`大写字母表`目录项
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------: | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“82H”） |
    // | 0x01     | 19               | 保留 |
    // | 0x14     | 4                | 起始簇号 |
    // | 0x18     | 8                | 文件大小 |

    // 创建一个新的`文件或目录`目录项
    //
    // 属性1
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------- | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“85H”） |
    // | 0x01     | 1                | 附属目录项数 |
    // | 0x02     | 2                | 校验和 |
    // | 0x04     | 4                | 文件属性 |
    // | 0x08     | 4                | 创建时间 |
    // | 0x0C     | 4                | 最后修改时间 |
    // | 0x10     | 4                | 最后访问时间 |
    // | 0x14     | 1                | 文件创建时间精确至10ms |
    // | 0x15     | 11               | 保留 |
    //
    // 属性2
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------- | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“C0H”） |
    // | 0x01     | 1                | 文件碎片标志（连续存放（无碎片）为“03H”，非连续存放（有碎片）为“01H”） |
    // | 0x02     | 1                | 保留 |
    // | 0x03     | 1                | 文件名字符数 |
    // | 0x04     | 2                | 文件名哈希值 |
    // | 0x06     | 2                | 保留 |
    // | 0x08     | 8                | 文件大小1 |
    // | 0x10     | 4                | 保留 |
    // | 0x14     | 4                | 起始簇号 |
    // | 0x18     | 8                | 文件大小2 |
    //
    // 属性3
    //
    // | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    // | :------- | :--------------- | :--------- |
    // | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“C1H”） |
    // | 0x01     | 1                | 保留 |
    // | 0x02     | 2N               | 文件名 |
    // | 0x02+2N  | 32-2-2N          | 保留 |

    /// 创建一个空目录项
    pub fn new_empty() -> Self {
        Self {
            entry_type: IndexEntryType::empty(),
            custom_defined: EntryCustom::default(),
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0; 32];

        bytes[0] = self.entry_type.bits();

        bytes[1..].copy_from_slice(unsafe { &self.custom_defined.raw });

        bytes
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let entry_type = IndexEntryType::from_bits_truncate(bytes[0]);

        let mut raw = [0u8; 31];
        raw.copy_from_slice(&bytes[1..]);

        Self {
            entry_type,
            custom_defined: EntryCustom {
                raw
            },
        }
    }
}

/// **目录项校验和**
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct IndexEntryChecksum(pub u16);

impl IndexEntryChecksum {
    /// 读取目录项，计算校验和
    pub fn add_entry(&mut self, entry_bytes: &[u8], is_first_entry: bool) {
        assert_eq!(entry_bytes.len(), 32);
        if is_first_entry {
            for i in 0..32 {
                match i {
                    2 | 3 => continue,
                    _ => {
                        self.0 >>= 1;
                        self.0 += 0x8000 * (self.0 & 1) + entry_bytes[i as usize] as u16;
                    }
                }
            }
        } else {
            for i in 0..32 {
                self.0 >>= 1;
                self.0 += 0x8000 * (self.0 & 1) + entry_bytes[i as usize] as u16;
            }
        }
    }
}

/// 时间戳
#[derive(Debug, Default, Copy, Clone)]
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