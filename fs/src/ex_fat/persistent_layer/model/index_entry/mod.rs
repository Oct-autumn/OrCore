//! fs/src/ex_fat/persistent_layer/model/index_entry/mod.rs
//! 
//! exFAT目录项

pub mod volume_label;
pub mod fs_file;
pub mod file_directory;

use alloc::vec::Vec;
use bitflags::bitflags;

use volume_label::VolumeLabelCostume;
use fs_file::FsFileCostume;
use file_directory::{
    FileAttributes,
    FileDirectoryCostume,
    FileDirectoryCostume1,
    FileDirectoryCostume2,
    FileDirectoryCostume3,
    FragmentFlag,
    TimeStamp};
use crate::ex_fat::persistent_layer::up_case_table::FileNameHash;
use super::{
    cluster_id::ClusterId,
    unicode_str::UnicodeString,
};

pub trait IndexEntryCostumeBytes {
    fn to_bytes(&self) -> [u8; 31];
    fn from_bytes(bytes: &[u8]) -> Self;
}

/// 目录项的派生项自定义部分
#[derive(Debug, Clone)]
pub enum EntryCostume {
    VolumeLabel(VolumeLabelCostume),
    AllocationBitmap(FsFileCostume),
    UpCaseTable(FsFileCostume),
    FileDirectory(FileDirectoryCostume),
    Empty,
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct IndexEntryType: u8 {
        /// 卷标
        const VolumnLabel       = 0x03;
        /// 簇分配位图
        const AllocationBitmap  = 0x01;
        /// 大写字母表
        const UpcaseTable       = 0x02;
        /// 文件或目录
        const FileDirectory     = 0x05;
        /// 流扩展
        const StreamExtension   = 0x40;
        /// 文件名
        const Filename          = 0x41;
        /// 使用中
        const InUse             = 0x80;

        /// 暂未使用
        const VolumnGUID        = 0x20;
        /// 暂未使用
        const TexFATPadding     = 0x21;
        /// 暂未使用
        const VendorExtension   = 0x60;
        /// 暂未使用
        const VendorAllocation  = 0x61;
    }
}

/// 目录项
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// 目录项类型
    pub entry_type: IndexEntryType,
    /// 本条目由派生的目录项定义
    pub custom_defined: EntryCostume,
}

impl IndexEntry {
    /// 创建一个新的`卷标`目录项
    ///
    /// 卷标使用Unicode字符集，每个字符占用2个字节，最多15个字符（30字节）
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------: | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“83H”） |
    /// | 0x01     | 1                | 卷标字符数 |
    /// | 0x02     | 22               | 卷标 |
    /// | 0x18     | 8                | 保留（也可用） |
    pub fn new_volume_label(volume_label: &UnicodeString) -> Self {
        let length = volume_label.len();
        if length > 15 {
            // 卷标大于30Byte，抛出异常
            panic!("Volume label is too long");
        }

        Self {
            entry_type: IndexEntryType::InUse | IndexEntryType::VolumnLabel,
            custom_defined: EntryCostume::VolumeLabel(VolumeLabelCostume::new(volume_label)),
        }
    }

    /// 创建一个新的`簇分配位图`目录项
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------: | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“81H”） |
    /// | 0x01     | 19               | 保留 |
    /// | 0x14     | 4                | 起始簇号 |
    /// | 0x18     | 8                | 文件大小 |
    pub fn new_allocation_bitmap(first_cluster: ClusterId, data_length: u64) -> Self {
        Self {
            entry_type: IndexEntryType::InUse | IndexEntryType::AllocationBitmap,
            custom_defined: EntryCostume::AllocationBitmap(FsFileCostume::new(first_cluster, data_length)),
        }
    }

    /// 创建一个新的`大写字母表`目录项
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------: | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“82H”） |
    /// | 0x01     | 19               | 保留 |
    /// | 0x14     | 4                | 起始簇号 |
    /// | 0x18     | 8                | 文件大小 |
    pub fn new_up_case_table(first_cluster: ClusterId, data_length: u64) -> Self {
        Self {
            entry_type: IndexEntryType::InUse | IndexEntryType::UpcaseTable,
            custom_defined: EntryCostume::UpCaseTable(FsFileCostume::new(first_cluster, data_length)),
        }
    }

    /// 创建一个新的`文件或目录`目录项
    ///
    /// 属性1
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------- | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“85H”） |
    /// | 0x01     | 1                | 附属目录项数 |
    /// | 0x02     | 2                | 校验和 |
    /// | 0x04     | 4                | 文件属性 |
    /// | 0x08     | 4                | 创建时间 |
    /// | 0x0C     | 4                | 最后修改时间 |
    /// | 0x10     | 4                | 最后访问时间 |
    /// | 0x14     | 1                | 文件创建时间精确至10ms |
    /// | 0x15     | 11               | 保留 |
    ///
    /// 属性2
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------- | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“C0H”） |
    /// | 0x01     | 1                | 文件碎片标志（连续存放（无碎片）为“03H”，非连续存放（有碎片）为“01H”） |
    /// | 0x02     | 1                | 保留 |
    /// | 0x03     | 1                | 文件名字符数 |
    /// | 0x04     | 2                | 文件名哈希值 |
    /// | 0x06     | 2                | 保留 |
    /// | 0x08     | 8                | 文件大小1 |
    /// | 0x10     | 4                | 保留 |
    /// | 0x14     | 4                | 起始簇号 |
    /// | 0x18     | 8                | 文件大小2 |
    ///
    /// 属性3
    ///
    /// | 字节偏移  | 字段长度（字节）    | 内容及含义 |
    /// | :------- | :--------------- | :--------- |
    /// | 0x00     | 1                | 目录项的类型（卷标目录项的特征值为“C1H”） |
    /// | 0x01     | 1                | 保留 |
    /// | 0x02     | 2N               | 文件名 |
    /// | 0x02+2N  | 32-2-2N          | 保留 |
    pub fn new_file_directory(
        file_attributes: FileAttributes,
        create_time_ms: usize,
        last_modify: usize,
        last_access: usize,
        is_fragment: bool,
        file_directory_name: UnicodeString,
        file_name_hash: FileNameHash,
        file_size: u64,
        first_cluster: ClusterId,
    ) -> Vec<Self> {
        let mut ret = Vec::new();

        if file_directory_name.len() > 255 {
            // 文件名大于255字符，抛出异常
            panic!("Name is too long");
        }

        // 附属目录项数
        // 应至少为2，根据文件名长度N，计算公式为：N/15(向上取整)+1
        let secondary_count = (file_directory_name.len() + 14) / 15 + 1;

        // 创建时间戳
        let (creat_time_stamp, creat_10ms_increment) = TimeStamp::from_unix_ms_timestamp(create_time_ms as u64);

        // 创建属性项1
        ret.push(Self {
            entry_type: IndexEntryType::InUse | IndexEntryType::FileDirectory,
            custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(FileDirectoryCostume1::new(
                secondary_count as u8,
                IndexEntryChecksum::default(),
                file_attributes,
                creat_time_stamp,
                TimeStamp::from_unix_timestamp(last_modify as u64),
                TimeStamp::from_unix_timestamp(last_access as u64),
                creat_10ms_increment,
            ))),
        });

        // 碎片标志
        let fragment_flag = if is_fragment {
            FragmentFlag::Fragmented
        } else {
            FragmentFlag::Continuous
        };

        // 创建属性项2
        ret.push(Self {
            entry_type: IndexEntryType::InUse | IndexEntryType::StreamExtension,
            custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume2(FileDirectoryCostume2::new(
                fragment_flag,
                file_directory_name.len() as u8,
                file_name_hash,
                file_size,
                first_cluster,
                file_size,
            ))),
        });

        // 创建属性项3
        for i in 0..secondary_count - 1 {
            let start = i * 15;
            let end = if i == secondary_count - 2 {
                file_directory_name.len()
            } else {
                (i + 1) * 15
            };

            ret.push(Self {
                entry_type: IndexEntryType::InUse | IndexEntryType::Filename,
                custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume3(FileDirectoryCostume3::new(
                    file_directory_name.slice(start, end)
                ))),
            });
        }

        // 计算并设置校验和
        let mut checksum = IndexEntryChecksum::default();
        for i in 0..ret.len() {
            checksum.add_entry(ret[i].to_bytes().as_slice(), i == 0);
        }

        if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(costume1)) = &mut ret[0].custom_defined {
            costume1.set_check_sum = checksum;
        }

        ret
    }
    
    /// 创建一个空目录项
    pub fn new_empty() -> Self {
        Self {
            entry_type: IndexEntryType::empty(),
            custom_defined: EntryCostume::Empty,
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0] = self.entry_type.bits();

        match self.custom_defined.clone() {
            EntryCostume::VolumeLabel(custom) => {
                bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
            }
            EntryCostume::AllocationBitmap(custom) => {
                bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
            }
            EntryCostume::UpCaseTable(custom) => {
                bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
            }
            EntryCostume::FileDirectory(custom) => {
                match custom {
                    FileDirectoryCostume::Costume1(custom) => {
                        bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
                    }
                    FileDirectoryCostume::Costume2(custom) => {
                        bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
                    }
                    FileDirectoryCostume::Costume3(custom) => {
                        bytes[1..].copy_from_slice(custom.to_bytes().as_slice());
                    }
                }
            }
            EntryCostume::Empty => {
                bytes[1..].fill(0);
            }
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let entry_type = IndexEntryType::from_bits(bytes[0])?;
        
        if entry_type.bits() == 0x00 {
            // 空目录项
            return Some(Self {
                entry_type,
                custom_defined: EntryCostume::Empty,
            });
        }

        let mut entry_type_without_inuse = entry_type;
        entry_type_without_inuse.set(IndexEntryType::InUse, false);

        if entry_type_without_inuse.0 == IndexEntryType::VolumnLabel.0 {
            let custom = VolumeLabelCostume::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::VolumeLabel(custom),
            })
        } else if entry_type_without_inuse.0 == IndexEntryType::AllocationBitmap.0 {
            let custom = FsFileCostume::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::AllocationBitmap(custom),
            })
        } else if entry_type_without_inuse.0 == IndexEntryType::UpcaseTable.0 {
            let custom = FsFileCostume::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::UpCaseTable(custom),
            })
        } else if entry_type_without_inuse.0 == IndexEntryType::FileDirectory.0 {
            let custom1 = FileDirectoryCostume1::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(custom1)),
            })
        } else if entry_type_without_inuse.0 == IndexEntryType::StreamExtension.0 {
            let custom2 = FileDirectoryCostume2::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume2(custom2)),
            })
        } else if entry_type_without_inuse.0 == IndexEntryType::Filename.0 {
            let custom3 = FileDirectoryCostume3::from_bytes(&bytes[1..]);
            Some(Self {
                entry_type,
                custom_defined: EntryCostume::FileDirectory(FileDirectoryCostume::Costume3(custom3)),
            })
        } else {
            panic!("Unknown entry type {:02X}", entry_type.bits());
        }
    }
}

/// **目录项校验和**
#[derive(Default, Debug, Clone)]
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

