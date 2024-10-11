//! fs/src/ex_fat/persistent_layer/cluster_bitmap.rs
//!
//! 簇分配位图

use crate::bitmap::Bitmap;
use crate::block_dev::BlockDevice;
use crate::ex_fat::persistent_layer::boot::BootSector;
use crate::ex_fat::persistent_layer::model::cluster_id::ClusterId;
use alloc::sync::Arc;

/// 簇分配位图
pub struct ClusterAllocBitmap {
    /// 位图
    pub cluster_bitmap: Bitmap,
    /// 存储设备接口指针
    pub device: Arc<dyn BlockDevice>,
}

impl ClusterAllocBitmap {
    pub fn new(boot_sector: &BootSector, device: &Arc<dyn BlockDevice>) -> Self {
        let bitmap_bytes = (boot_sector.cluster_count + 7) / 8;
        let bytes_per_sector = 1u32 << boot_sector.bytes_per_sector_shift;
        let bitmap_blocks: usize = ((bitmap_bytes + bytes_per_sector - 1) >> boot_sector.bytes_per_sector_shift) as usize;

        /*
            以上代码等同于：
            let cluster_count = boot_sector.cluster_count.into();
            let bytes_per_sector = 1u32 << boot_sector.bytes_per_sector_shift;
            let bitmap_bytes = (cluster_count + 7) / 8;
            let bitmap_blocks: usize = bitmap_bytes / bytes_per_sector;
        */

        Self {
            cluster_bitmap: Bitmap::new(
                boot_sector.cluster_heap_offset as usize,
                bitmap_blocks,
            ),
            device: device.clone(),
        }
    }
    
    pub fn init(&mut self) {
        self.cluster_bitmap.init(&self.device);
    }

    /// 分配一个簇
    pub fn alloc(&mut self) -> Option<ClusterId> {
        self.cluster_bitmap
            .alloc(&self.device)
            .map(|id| ClusterId::from(id as u32 + 2))
        // 由于FAT表的簇号从2开始，所以这里要加2
    }

    /// 释放一个簇
    pub fn free(&mut self, cluster: &ClusterId) {
        self.cluster_bitmap.free(&self.device, cluster.0 as usize - 2);
        // 由于FAT表的簇号从2开始，所以这里要减2
    }

    /// 判断簇是否被使用
    pub fn is_using(&self, cluster: &ClusterId) -> bool {
        self.cluster_bitmap
            .is_used(&self.device, cluster.0 as usize - 2)
        // 由于FAT表的簇号从2开始，所以这里要减2
    }
}
