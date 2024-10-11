//! fs/src/ex_fat/persistent_layer/fat.rs
//!
//! 实现FAT表

use alloc::sync::Arc;

use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::ex_fat::persistent_layer::boot::BootSector;
use crate::ex_fat::persistent_layer::model::cluster_id::ClusterId;

pub struct FileAllocationTable {
    /// 扇区大小描述
    bytes_per_sector_shift: u8,
    /// FAT起始扇区
    start_sector: u32,
    /// FAT长度（单位：扇区）
    length: u32,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
}

impl FileAllocationTable {
    /// 创建FAT实例
    pub fn new(boot_sector: &BootSector, device: &Arc<dyn BlockDevice>) -> Self {
        Self {
            bytes_per_sector_shift: boot_sector.bytes_per_sector_shift,
            start_sector: boot_sector.fat_offset.into(),
            length: boot_sector.fat_length.into(),
            device: device.clone(),
        }
    }

    /// 初始化存储设备上的FAT表
    ///
    /// mut仅用于标记此操作会修改FAT表
    pub fn init_fat_on_device(&mut self) {
        // 初始化FAT表头
        let first_sector = get_block_cache(self.start_sector as usize, &self.device);
        first_sector.write().modify_and_sync(0, |data: &mut [u32; 128]| {
            data[0] = 0xFFFFFFF8; // FAT表头
            data[1] = 0xFFFFFFFF; // FAT表头
            data[2..].fill(0);  // 剩余项初始化为0
        });
        // 遍历FAT表的每个扇区
        for i in 1..self.length {
            let sector = get_block_cache((self.start_sector + i) as usize, &self.device);
            sector.write().modify_and_sync(0, |data: &mut [u64; 64]| {
                data.fill(0);   // 初始化为0
            });
        }
    }

    /// 检查FAT表是否有效
    pub fn check_validate(&self) -> bool {
        // 检查FAT表头
        let first_sector = get_block_cache(self.start_sector as usize, &self.device);
        if first_sector.read().read(0, |entry: &u32| *entry != 0xFFFFFFF8)
            || first_sector.read().read(4, |entry: &u32| *entry != 0xFFFFFFFF)
        {
            return false;
        }

        true
    }

    /// 获取指定FAT表项的值
    ///
    /// # 参数
    ///   `cluster_id`: u32, 簇号
    ///
    /// # 返回
    ///   u32, FAT表项的值
    fn get_entry(&self, cluster_id: &ClusterId) -> Option<u32> {
        let (sector_id, offset) = self.translate_cluster_id(cluster_id);

        //检查簇号是否有效
        if sector_id >= self.start_sector + self.length {
            // 簇号超出FAT表范围
            return None;
        }

        // 读取FAT表项
        let block_cache = get_block_cache(sector_id as usize, &self.device);
        let entry = block_cache.read().read(offset as usize, |entry: &u32| *entry);

        Some(entry)
    }

    /// 修改指定FAT表项的值
    ///
    /// mut仅用于标记此操作会修改FAT表
    fn set_entry(&mut self, cluster_id: &ClusterId, next_cluster_id: &ClusterId) -> Option<()> {
        let (sector_id, offset) = self.translate_cluster_id(cluster_id);

        //检查簇号是否有效
        if sector_id >= self.start_sector + self.length {
            // 簇号超出FAT表范围
            return None;
        }

        // 修改FAT表项
        get_block_cache(sector_id as usize, &self.device)
            .write()
            .modify(offset as usize, |entry: &mut u32| {
                *entry = next_cluster_id.0
            });

        Some(())
    }

    /// 将簇号转换为FAT表项的扇区号与偏移
    fn translate_cluster_id(&self, cluster_id: &ClusterId) -> (u32, u32) {
        let sector_id = self.start_sector + (cluster_id.0 >> (self.bytes_per_sector_shift - 2));
        let offset = (cluster_id.0 & (((1 << self.bytes_per_sector_shift) >> 2) - 1)) << 2;

        /*
            以上代码等同于：
            // 计算FAT表项所在的扇区与偏移
            // 扇区大小
            let sector_bytes = 1 << self.bytes_per_sector_shift;
            // 每个扇区的FAT表项数量
            let entry_per_sector = sector_bytes / 4;
            // FAT表项所在的扇区号
            let sector_id = self.start_sector + cluster_id / entry_per_sector;
            // FAT表项在扇区内的偏移
            let offset = (cluster_id % entry_per_sector) * 4;
        */

        (sector_id, offset)
    }

    /// 获取文件的下一个簇号
    pub fn get_next_cluster(&self, cluster_id: &ClusterId) -> Option<ClusterId> {
        if cluster_id.is_free() || cluster_id.is_bad_cluster() || cluster_id.is_end_of_file() {
            // 未分配簇或坏簇或结束标志
            None
        } else {
            // 查找下一个簇号
            let fat_entry = self.get_entry(&cluster_id)?;
            Some(ClusterId(fat_entry))
        }
    }

    /// 设置文件的下一个簇号
    ///
    /// mut仅用于标记此操作会修改位图内容
    pub fn set_next_cluster(
        &mut self,
        cluster_id: &ClusterId,
        next_cluster_id: &ClusterId,
    ) -> Option<()> {
        if cluster_id.is_free() || cluster_id.is_bad_cluster() || cluster_id.is_end_of_file() {
            // 未分配簇或坏簇或最后一个簇
            None
        } else {
            // 设置下一个簇号
            self.set_entry(&cluster_id, &next_cluster_id)
        }
    }
}
