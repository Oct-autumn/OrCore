//! fs/src/ex_fat/persistent_layer/mod.rs
//!
//! 簇管理器

pub mod model;
pub mod boot;
pub mod fat;
pub mod up_case_table;
pub mod cluster_bitmap;

use crate::block_cache::{get_block_cache, BlockCache};
use crate::ex_fat::persistent_layer::model::cluster_id::ClusterId;
use crate::{config, BlockDevice};
use alloc::sync::Arc;
use alloc::vec::Vec;
use boot::BootSector;
use cluster_bitmap::ClusterAllocBitmap;
use fat::FileAllocationTable;
use spin::RwLock;


/// 簇管理器，管理簇的分配、释放、访问
pub struct ClusterManager {
    /// 首簇起始位置
    pub cluster_heap_offset: u32,
    /// 每簇扇区数
    pub sectors_per_cluster: u32,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
    /// FAT表
    file_allocation_table: FileAllocationTable,
    /// 簇分配位图
    cluster_alloc_bitmap: ClusterAllocBitmap,
}

impl ClusterManager {
    pub fn new(boot_sector: &BootSector, device: &Arc<dyn BlockDevice>) -> Self {
        let file_allocation_table = FileAllocationTable::new(boot_sector, device);
        let cluster_alloc_bitmap = ClusterAllocBitmap::new(boot_sector, device);
        Self {
            cluster_heap_offset: boot_sector.cluster_heap_offset,
            sectors_per_cluster: 1 << boot_sector.sectors_per_cluster_shift,
            device: device.clone(),
            file_allocation_table,
            cluster_alloc_bitmap,
        }
    }

    /// 初始化簇管理器
    ///
    /// 该方法会初始化FAT表和簇分配位图，相当于格式化簇管理器。仅应在格式化文件系统时调用。
    pub fn init(&mut self) {
        self.file_allocation_table.init_fat_on_device();
        self.cluster_alloc_bitmap.init();

        // 申请一个簇(簇号为2)给簇分配位图使用
        if let Some(cluster_id) = self.cluster_alloc_bitmap.alloc() {
            // 2. 设置FAT表项为EOF
            self.file_allocation_table
                .set_next_cluster(&cluster_id, &ClusterId::eof());
            // 3. 清空簇数据
            self.clear_cluster(&cluster_id);
            let Some(cluster_id) = self.cluster_alloc_bitmap.alloc() else {
                unreachable!("This shouldn't happen");
            };
            assert_eq!(cluster_id.0, 2);    // 簇号应为2
        }
    }

    /// 清空簇数据
    fn clear_cluster(&mut self, cluster_id: &ClusterId) {
        let mut cluster_offset = 0;
        while let Some(cluster_sector) = self.get_cluster_sector(cluster_id, cluster_offset) {
            let mut cluster_sector = cluster_sector.write();
            cluster_sector.modify(0, |data: &mut [u8; config::SECTOR_BYTES]| {
                data.fill(0);
            });
            cluster_offset += 1;
            if cluster_offset >= self.sectors_per_cluster {
                break;
            }
        }
    }

    /// 检查簇是否为最后一簇
    fn is_last_cluster(&self, cluster_id: &ClusterId) -> bool {
        self.file_allocation_table.get_next_cluster(cluster_id) == Some(ClusterId::eof())
    }

    /// 释放当前簇的下一簇
    fn free_next_cluster(&mut self, cluster_id: &ClusterId) -> Option<()> {
        let next_cluster_id = self.file_allocation_table.get_next_cluster(cluster_id).unwrap();
        // 检查下一簇是否被分配
        // 簇能被释放的条件：
        //   是最后一簇 -> FAT表项为EOF
        if self.cluster_alloc_bitmap.is_using(&next_cluster_id)
            && self.is_last_cluster(&next_cluster_id)
        {
            // 设置下一簇的FAT表项为FREE状态
            self.file_allocation_table
                .set_next_cluster(&next_cluster_id, &ClusterId::free());
            // 释放下一簇
            self.cluster_alloc_bitmap.free(&next_cluster_id);
            // 设置当前簇的FAT表项为EOF
            self.file_allocation_table
                .set_next_cluster(cluster_id, &ClusterId::eof());
            return Some(());
        }
        None
    }

    /// 释放簇链
    ///
    /// 若cluster_id指向的不是簇链的首簇，会造成前一簇FAT表项指向未分配的簇。这会导致意料之外的行为，因此不建议在非首簇上调用该方法。
    pub fn free_cluster_chain(&mut self, cluster_id: ClusterId) {
        // 流程：
        // 1. 获取簇链
        // 2. 逐簇释放
        // 3. 释放首簇

        // 获取簇链
        let mut cluster_id_list = {
            let mut cluster_id_list = Vec::new();
            let mut current_cluster = cluster_id;
            loop {
                cluster_id_list.push(current_cluster);
                if let Some(next_cluster) = self.get_next_cluster(&current_cluster) {
                    if next_cluster.is_end_of_file() {
                        break;
                    }
                    current_cluster = next_cluster;
                } else {
                    break;
                }
            }
            cluster_id_list
        };
        cluster_id_list.pop();  // 弹出最后一簇，因为最后一簇是由它的上一簇指向删除的
        while let Some(cluster_id) = cluster_id_list.pop() {
            self.free_next_cluster(&cluster_id);
        }
        // 只剩第一簇还没释放
        self.cluster_alloc_bitmap.free(&cluster_id);
        // 因为我们很难找到指向它的簇，所以只能将其标记为未分配，而不能修改指向它的FAT表项
        // TODO：这或许可以通过整理FAT表来实现
    }


    /// 申请新的簇
    pub fn alloc_new_cluster(&mut self) -> Option<ClusterId> {
        // 流程：
        // 1. 申请簇号
        // 2. 设置FAT表项为EOF
        // 3. 清空簇数据

        // 1. 申请簇号
        if let Some(cluster_id) = self.cluster_alloc_bitmap.alloc() {
            // 2. 设置FAT表项为EOF
            self.file_allocation_table
                .set_next_cluster(&cluster_id, &ClusterId::eof());
            // 3. 清空簇数据
            self.clear_cluster(&cluster_id);
            Some(cluster_id)
        } else {
            None
        }
    }

    /// 为簇链申请并附加新簇
    ///
    /// 只支持在簇链的最后一簇上调用，非最后一簇会拒绝附加并返回None
    pub fn alloc_and_append_cluster(&mut self, cluster_id: &ClusterId) -> Option<ClusterId> {
        if self.is_last_cluster(cluster_id) {
            let new_cluster_id = self.alloc_new_cluster()?;
            
            // 如果是碎片，设置标志位
            //if new_cluster_id.0 - cluster_id.0 != 1 {
            //    *is_fragment = true;
            //}
            // 设置当前簇的FAT表项指向新簇
            self.file_allocation_table
                .set_next_cluster(cluster_id, &new_cluster_id);
            // 设置新簇的FAT表项为EOF
            self.file_allocation_table
                .set_next_cluster(&new_cluster_id, &ClusterId::eof());
            
            Some(new_cluster_id)
        } else {
            panic!("Not the last cluster");
        }
    }

    /// 获取下一簇的簇号
    pub fn get_next_cluster(&self, cluster_id: &ClusterId) -> Option<ClusterId> {
        self.file_allocation_table.get_next_cluster(cluster_id)
    }

    /// 获取簇号的首个扇区号
    fn get_cluster_first_sector(&self, cluster_id: &ClusterId) -> u32 {
        self.cluster_heap_offset + (cluster_id.0 - 2) * self.sectors_per_cluster
    }

    /// 获取指定簇的指定扇区偏移的缓存
    ///
    /// 当簇偏移超出当前簇的扇区范围时，会自动查找下一簇
    pub fn get_cluster_sector(&self, cluster_id: &ClusterId, cluster_offset: u32) -> Option<Arc<RwLock<BlockCache>>> {
        assert_ne!(cluster_id.0, 0);    // 不能获取保留簇

        if cluster_offset >= self.sectors_per_cluster {
            // 超出该簇的扇区范围，尝试查找目标簇
            let mut cluster_id = cluster_id.clone();
            let mut cluster_offset = cluster_offset;
            while let Some(next_cluster_id) = self.file_allocation_table.get_next_cluster(&cluster_id) {
                cluster_id = next_cluster_id;
                cluster_offset -= self.sectors_per_cluster;
                if cluster_offset < self.sectors_per_cluster {
                    return Some(get_block_cache((self.get_cluster_first_sector(&cluster_id) + cluster_offset) as usize, &self.device));
                }
            }
            None
        } else {
            // 未超出该簇的扇区范围，直接获取
            Some(get_block_cache((self.get_cluster_first_sector(cluster_id) + cluster_offset) as usize, &self.device))
        }
    }
}