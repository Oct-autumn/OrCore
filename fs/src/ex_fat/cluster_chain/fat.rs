//! fs/src/ex_fat/cluster_chain/fat.rs
//!
//! 实现FAT表

use crate::block_device::block_cache::BlockCacheManager;
use crate::ex_fat::boot_sector::BootSector;
use crate::ex_fat::model::cluster_id::ClusterId;
use alloc::sync::Arc;
use spin::Mutex;
use crate::config;

pub struct FileAllocationTable {
    /// 扇区大小描述
    pub bytes_per_sector_shift: u8,
    /// FAT起始扇区
    pub start_sector: usize,
    /// FAT长度（单位：扇区）
    pub length: usize,
    /// 缓存管理器
    block_cache_manager: Arc<Mutex<BlockCacheManager>>,
}

impl FileAllocationTable {
    /// 创建FAT实例
    pub fn new(boot_sector: &BootSector, block_cache_manager: Arc<Mutex<BlockCacheManager>>) -> Self {
        Self {
            bytes_per_sector_shift: boot_sector.bytes_per_sector_shift,
            start_sector: boot_sector.fat_offset as usize,
            length: boot_sector.fat_length as usize,
            block_cache_manager,
        }
    }

    /// 初始化存储设备上的FAT表
    pub fn init_fat_on_device(&self) {
        // <---- BCM 独占区 开始 ---->
        let mut bcm_guard = self.block_cache_manager.lock();
        // 清空FAT表的所有扇区
        for i in 0..self.length {
            let sector = bcm_guard.get_block_cache(self.start_sector + i);
            sector.write().modify_and_sync(0, |data: &mut [usize; config::SECTOR_BYTES / size_of::<usize>()]| {
                data.fill(0);   // 初始化为0
            });
        }
        // 初始化FAT表头
        let first_sector = bcm_guard.get_block_cache(self.start_sector);
        first_sector.write().modify_and_sync(0, |data: &mut [u32; 2]| {
            data[0] = 0xFFFFFFF8; // FAT表头
            data[1] = 0xFFFFFFFF; // FAT表头
        });
        // <---- BCM 独占区 结束 ---->
    }

    /// 检查FAT表是否有效
    pub fn check_validate(&self) -> bool {
        // 检查FAT表头
        let first_sector = self.block_cache_manager.lock().get_block_cache(self.start_sector);

        first_sector.read().read(0, |entry: &u32| *entry == 0xFFFFFFF8)
            && first_sector.read().read(4, |entry: &u32| *entry == 0xFFFFFFFF)
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
            panic!("ClusterId out of range");
        }

        // 读取FAT表项
        let block_cache = self.block_cache_manager.lock().get_block_cache(sector_id);
        let entry = block_cache.read().read(offset, |entry: &u32| *entry);

        Some(entry)
    }

    /// 修改指定FAT表项的值
    fn set_entry(&self, cluster_id: &ClusterId, next_cluster_id: &ClusterId) -> Option<()> {
        let (sector_id, offset) = self.translate_cluster_id(cluster_id);

        //检查簇号是否有效
        if sector_id >= self.start_sector + self.length {
            // 簇号超出FAT表范围
            return None;
        }

        // 修改FAT表项
        self.block_cache_manager.lock().get_block_cache(sector_id)
            .write()
            .modify(offset, |entry: &mut u32| {
                *entry = next_cluster_id.0
            });

        Some(())
    }

    /// 将簇号转换为FAT表项的扇区号与偏移
    fn translate_cluster_id(&self, cluster_id: &ClusterId) -> (usize, usize) {
        let sector_id = self.start_sector + (cluster_id.0 as usize >> (self.bytes_per_sector_shift - 2));
        let offset = (cluster_id.0 as usize & (((1 << self.bytes_per_sector_shift) >> 2) - 1)) << 2;

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
        if cluster_id.is_invalid() {
            // 无效簇号
            None
        } else {
            // 查找下一个簇号
            Some(ClusterId(self.get_entry(&cluster_id)?))
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
        if cluster_id.is_invalid() {
            // 无效簇号
            None
        } else {
            // 设置下一个簇号
            self.set_entry(&cluster_id, &next_cluster_id)
        }
    }
}
