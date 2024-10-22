//! fs/src/ex_fat/cluster_chain/mod.rs
//!
//! 簇管理器

pub mod fat;
pub mod cluster_bitmap;
mod bitmap;

use crate::block_device::block_cache::{BlockCache, BlockCacheManager};
use crate::ex_fat::boot_sector::BootSector;
use crate::ex_fat::model::cluster_id::ClusterId;
use alloc::sync::Arc;
use cluster_bitmap::ClusterAllocBitmap;
use fat::FileAllocationTable;
use spin::{Mutex, RwLock};

/// 簇链管理器，管理簇链的申请、扩展、销毁、存取
pub struct ClusterManager {
    /// 首簇起始扇区号
    pub cluster_heap_offset: usize,
    /// 每簇扇区数
    pub sectors_per_cluster: usize,
    /// 缓存管理器
    block_cache_manager: Arc<Mutex<BlockCacheManager>>,
    /// FAT表
    file_allocation_table: FileAllocationTable,
    /// 簇分配位图
    cluster_alloc_bitmap: ClusterAllocBitmap,
}

/*
    簇链管理应实现：
        申请（申请新簇）
        扩展（申请新簇并在簇链尾部追加）
            若申请的新簇簇号不连续，则需要将整个簇链转化为非连续簇链
        销毁（从簇链尾部逐级销毁簇链）（簇链头、是否为连续簇）
            若为连续簇链，则仅需取消簇分配位图中的分配
            若为离散簇链，则需同时删除FAT表和分配位图中的分配
        存取（簇链头，簇链内偏移，是否为连续簇）
*/


impl ClusterManager {
    pub fn new(boot_sector: &BootSector, block_cache_manager: Arc<Mutex<BlockCacheManager>>) -> Self {
        let file_allocation_table = FileAllocationTable::new(boot_sector, block_cache_manager.clone());
        let cluster_alloc_bitmap = ClusterAllocBitmap::new(boot_sector, block_cache_manager.clone());
        
        Self {
            cluster_heap_offset: boot_sector.cluster_heap_offset as usize,
            sectors_per_cluster: 1 << boot_sector.sectors_per_cluster_shift,
            block_cache_manager,
            file_allocation_table,
            cluster_alloc_bitmap,
        }
    }

    /// 检查FAT表是否有效
    ///
    /// 用于从设备中加载文件系统时检查FAT表是否有效
    pub fn check_fat(&self) -> bool {
        self.file_allocation_table.check_validate()
    }

    /// 清空簇数据
    fn clear_cluster(&mut self, cluster_id: &ClusterId) {
        let cluster_first_sector_id = self.cluster_heap_offset + (cluster_id.0 as usize - 2) * self.sectors_per_cluster;
        for i in 0..self.sectors_per_cluster {
            self.block_cache_manager.lock().direct_set_zero(cluster_first_sector_id + i);
        }
    }

    /// 申请新的簇链
    ///
    /// # param
    /// - size: 新簇链长度
    /// - cluster_id: 指定新簇链的首簇号，若为EOF，则将分配新簇链的首簇号
    /// # retval
    /// - Option<(ClusterId, bool)>: 新簇链的首簇号和新簇链（及其自身相对于原簇链）是否为非连续簇链
    pub fn alloc_new_cluster(&mut self, cluster_id: &ClusterId, size: usize, is_fragment: bool) -> Option<(ClusterId, bool)> {
        // 流程：
        // 1. 检查是否有空闲簇，以及空闲簇数量是否满足分配需求
        // 2. 申请第一簇
        //  2.1. 申请簇
        //  2.2. 检查申请的簇号是否与指定的簇号相同
        //   2.2.1. 若指定的簇号为EOF，则重设首簇号
        //   2.2.2. 若指定的簇号不为EOF，则申请的簇号应与指定的簇号相同，不相同则重设首簇号，并标记为非连续簇链
        //  2.3. 清空簇
        //  2.4. 设定FAT表项
        // 3. 循环申请新簇，直到申请完毕
        //  3.1. 申请簇
        //  3.2. 根据簇号是否连续，决定是否需要将之前的簇链转化为非连续簇链
        //  3.3. 清空簇

        // 1. 检查是否有空闲簇，以及空闲簇数量是否满足分配需求
        if self.cluster_alloc_bitmap.used_cluster_count + size > self.cluster_alloc_bitmap.cluster_count {
            panic!("No enough cluster to alloc");
        }

        // 返回值：（新簇链的首簇号，新簇链（及其自身相对于原簇链）是否为非连续簇链）
        let mut ret = (cluster_id.clone(), is_fragment);

        // 还需要申请的簇数
        let mut cluster_chain_len = 0;
        // 当前簇号
        let mut now_cluster_id = ClusterId::eof();

        // 2. 申请第一簇
        now_cluster_id = self.cluster_alloc_bitmap.alloc(&ret.0).unwrap();
        if ret.0.is_eof() {
            // 若指定的簇号为EOF（即不指定首簇号），则重设首簇号，但不需要标记为非连续簇链
            ret.0 = now_cluster_id.clone();
        } else if now_cluster_id != ret.0 {
            // 若指定的簇号不为EOF，且申请的簇号与之不一致，则说明新簇号不连续
            // 重设首簇号，并标记为非连续簇链
            ret.0 = now_cluster_id.clone();
            ret.1 = true;
        }
        // 清空簇
        self.clear_cluster(&now_cluster_id);
        // 若为非连续簇链，设定FAT表项
        if ret.1 {
            self.file_allocation_table.set_next_cluster(&now_cluster_id, &ClusterId::eof());
        }
        cluster_chain_len += 1;

        // 3. 循环申请新簇，直到申请完毕
        while cluster_chain_len < size {
            let target_cluster_id = now_cluster_id.0 + 1;
            // 申请新簇
            let new_cluster_id = self.cluster_alloc_bitmap.alloc(&ClusterId(target_cluster_id)).unwrap();
            // 检查新簇号是否连续
            if target_cluster_id != new_cluster_id.0 && !ret.1 {
                // 若新簇号不连续，则需要将之前的簇链转化为非连续簇链
                // 转化簇链
                self.set_continued_cluster_chain(&ret.0, cluster_chain_len);
                // 设置连续标志位
                ret.1 = true;
            }

            // 清空簇
            self.clear_cluster(&now_cluster_id);

            // 若为非连续簇链，则需设置FAT表项（当前簇指向下一簇，下一簇指向EOF）
            if ret.1 {
                self.file_allocation_table.set_next_cluster(&now_cluster_id, &new_cluster_id);
                self.file_allocation_table.set_next_cluster(&new_cluster_id, &ClusterId::eof());
            }

            cluster_chain_len += 1;

            // 更新当前簇号
            now_cluster_id = new_cluster_id;
        }

        // 修改了FAT、簇分配位图，需要同步
        self.block_cache_manager.lock().sync_all();

        Some(ret)
    }

    /// 在FAT中设置一段连续簇链
    pub fn set_continued_cluster_chain(&mut self, cluster_id: &ClusterId, len: usize) {
        assert!(len > 0);
        let mut now_cluster_id = cluster_id.clone();
        let mut len = len;

        // 从指定簇开始，设置连续簇链
        while len > 1 {
            if self.file_allocation_table.set_next_cluster(&now_cluster_id, &ClusterId(now_cluster_id.0 + 1)).is_none() {
                panic!("Unable to set next cluster");
            }
            now_cluster_id.0 += 1;
            len -= 1;
        }

        // 设置最后一个簇的下一个簇为EOF
        if self.file_allocation_table.set_next_cluster(&now_cluster_id, &ClusterId::eof()).is_none() {
            panic!("Unable to set next cluster");
        }
    }
    
    /// 在FAT中设置某簇的下一簇
    pub fn set_next_cluster(&mut self, cluster_id: &ClusterId, next_cluster_id: &ClusterId) {
        if self.file_allocation_table.set_next_cluster(cluster_id, next_cluster_id).is_none() {
            panic!("Unable to set next cluster");
        }
    }

    /// 释放簇链
    ///
    /// # param
    /// - cluster_id: 簇链首簇号（由调用者保证必须为首簇）
    /// - size: 簇链长度
    /// - is_fragment: 是否为非连续簇链
    pub fn free_cluster_chain(&mut self, cluster_id: &ClusterId, size: usize, is_fragment: bool) -> Option<()> {
        // 流程：
        // 1. 检查首簇号是否有效
        // 2. 根据簇链是否连续，决定释放方式
        //  2.1. 若为非连续簇链，则迭代查找下一簇并释放
        //  2.2. 若为连续簇链，则直接释放簇
        
        // 1. 检查首簇号是否有效
        if !self.cluster_alloc_bitmap.is_allocated(cluster_id) {
            panic!("Cluster chain not allocated");
        }
        if cluster_id.is_invalid() {
            panic!("Invalid cluster id");
        }
        
        // 2. 根据簇链是否连续，决定释放方式
        if is_fragment {
            // 非连续簇链
            let mut now_cluster_id = cluster_id.clone();
            for _ in 0..size {
                let next_cluster_id = self.file_allocation_table.get_next_cluster(&now_cluster_id);
                self.cluster_alloc_bitmap.free(&now_cluster_id);
                now_cluster_id = next_cluster_id.unwrap();
            }
        } else {
            // 连续簇链
            let mut now_cluster_id = cluster_id.clone();
            for _ in 0..size {
                self.cluster_alloc_bitmap.free(&now_cluster_id);
                now_cluster_id.0 += 1;
            }
        }

        // 修改了FAT、簇分配位图，需要同步
        self.block_cache_manager.lock().sync_all();
        
        Some(())
    }

    /// 根据簇号和簇内偏移获取簇内扇区
    pub fn get_cluster_sector(&self, cluster_id: &ClusterId, cluster_offset: usize) -> Option<Arc<RwLock<BlockCache>>> {
        if cluster_offset > self.sectors_per_cluster {
            panic!("Cluster offset out of range");
        }
        // 扇区号：首簇扇区号 + （簇号 - 2） * 每簇扇区数 + 簇内偏移
        let cluster_sector_offset = self.cluster_heap_offset + (cluster_id.0 as usize - 2) * self.sectors_per_cluster + cluster_offset;
        Some(self.block_cache_manager.lock().get_block_cache(cluster_sector_offset))
    }
    
    /// 获取簇链的下一簇
    pub fn get_next_cluster(&self, cluster_id: &ClusterId) -> Option<ClusterId> {
        self.file_allocation_table.get_next_cluster(cluster_id)
    }
}