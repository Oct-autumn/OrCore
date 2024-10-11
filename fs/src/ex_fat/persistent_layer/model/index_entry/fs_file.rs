//! fs/src/ex_fat/persistent_layer/model/index_entry/fs_file.rs
//! 
//! 文件系统文件专用目录项

use core::fmt::{Debug, Formatter};

use super::super::{
    cluster_id::ClusterId,
    index_entry::IndexEntryCostumeBytes,
};

/// 文件系统文件专用目录项
///
/// 用于索引exFAT专用文件：簇分配位图、大写字母表
#[repr(C)]
#[derive(Clone)]
pub struct FsFileCostume {
    /// 起始簇号
    first_cluster: ClusterId,
    /// 文件大小（单位：字节）
    data_length: u64,
}

impl FsFileCostume {
    pub fn new(first_cluster: ClusterId, data_length: u64) -> Self {
        Self {
            first_cluster,
            data_length,
        }
    }
}

impl IndexEntryCostumeBytes for FsFileCostume {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];
        arr[19..23].copy_from_slice(&self.first_cluster.0.to_le_bytes());
        arr[23..31].copy_from_slice(&self.data_length.to_le_bytes());
        arr
    }

    fn from_bytes(arr: &[u8]) -> Self {
        assert_eq!(arr.len(), 31);
        // 检查must_be_zero字段是否全为0
        for i in 0..19 {
            if arr[i] != 0 {
                panic!("must_be_zero字段不全为0");
            }
        }
        Self {
            first_cluster: ClusterId(<u32>::from_le_bytes([arr[19], arr[20], arr[21], arr[22]])),
            data_length: <u64>::from_le_bytes([arr[23], arr[24], arr[25], arr[26], arr[27], arr[28], arr[29], arr[30]]),
        }
    }
}

impl Debug for FsFileCostume {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FsIndexCostume")
            .field("first_cluster", &self.first_cluster)
            .field("data_length", &self.data_length)
            .finish()
    }
}