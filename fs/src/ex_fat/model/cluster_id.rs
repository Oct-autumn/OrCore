//! fs/src/ex_fat/cluster_chain/model/cluster_id.rs
//!
//! 簇ID 兼 FAT表项

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterId(pub u32);

impl ClusterId {
    // 快速创建
    /// 快速创建一个文件结束标志
    pub fn eof() -> Self {
        Self(0xFFFFFFFF)
    }
    /// 快速创建一个坏簇标志
    pub fn bad_cluster() -> Self {
        Self(0xFFFFFFF7)
    }
    /// 快速创建一个空闲标志
    pub fn free() -> Self {
        Self(0x00000000)
    }

    // 快速判断
    /// 判断表项是否为文件结束标志
    pub fn is_eof(&self) -> bool { self.0 == 0xFFFFFFFF }
    /// 判断表项是否为坏簇标志
    pub fn is_bad_cluster(&self) -> bool {
        self.0 == 0xFFFFFFF7
    }
    /// 判断表项是否为空闲
    pub fn is_free(&self) -> bool {
        self.0 == 0x00000000
    }
    /// 是否为无效簇号
    pub fn is_invalid(&self) -> bool {
        self.is_eof() || self.is_bad_cluster() || self.is_free()
    }
}

impl From<u32> for ClusterId {
    fn from(entry: u32) -> Self {
        Self(entry)
    }
}

impl From<ClusterId> for u32 {
    fn from(entry: ClusterId) -> Self {
        entry.0
    }
}

impl Default for ClusterId {
    fn default() -> Self {
        Self::free()
    }
}